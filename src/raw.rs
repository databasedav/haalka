//! [haalka](crate)'s core abstraction, allowing one to rig any [`Entity`] with ergonomic
//! [`futures_signals::Signal`](https://docs.rs/futures-signals/latest/futures_signals/signal/trait.Signal.html) driven reactivity, including
//! methods for registering children, [`Component`]s, event listeners (via observers), and
//! [`System`]s all using a declarative builder pattern/[fluent interface](https://en.wikipedia.org/wiki/Fluent_interface).
//! Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`raw_el`](https://github.com/MoonZoon/MoonZoon/tree/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/raw_el).

use std::{
    future::Future,
    marker::PhantomData,
    mem,
    sync::{Arc, OnceLock},
};

use super::{
    node_builder::{NodeBuilder, TaskHolder, async_world},
    raw::utils::remove_system_holder_on_remove,
};
use apply::Apply;
use bevy_ecs::{component::*, error::*, prelude::*, system::*, world::*};
use bevy_log::error;
use bevy_tasks::Task;
use bevy_utils::prelude::*;
use enclose::enclose as clone;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};
use haalka_futures_signals_ext::SignalExtBool;

/// A thin layer over a [`NodeBuilder`] that exposes higher level ECS related methods.
/// Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`RawHtmlElement`](https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/raw_el/raw_html_el.rs).
pub struct RawHaalkaEl {
    node_builder: Option<NodeBuilder>,
    deferred_updaters: Vec<Box<dyn FnOnce(RawHaalkaEl) -> RawHaalkaEl + Send + 'static>>,
}

impl From<NodeBuilder> for RawHaalkaEl {
    fn from(node_builder: NodeBuilder) -> Self {
        Self {
            node_builder: Some(node_builder),
            ..Self::new_dummy()
        }
    }
}

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            node_builder: Some(NodeBuilder::from(node_bundle)),
            ..Self::new_dummy()
        }
    }
}

impl Default for RawHaalkaEl {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether to append the deferred updater to the front or the back *of the back* of the line. See
/// [`RawHaalkaEl::defer_update`].
#[allow(missing_docs)]
pub enum DeferredUpdaterAppendDirection {
    Front,
    Back,
}

impl RawHaalkaEl {
    fn new_dummy() -> Self {
        Self {
            node_builder: None,
            deferred_updaters: default(),
        }
    }

    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self {
            node_builder: Some(default()),
            deferred_updaters: default(),
        }
    }

    // TODO: allow attaching some sortable data to unlimit ordering options (this prolly won't work
    // because it would pollute the struct with a type?)
    /// Force updates to be in the back of the line, with some crude ordering.
    ///
    /// Useful when updates must be applied after some node is wrapped with another.
    ///
    /// # Notes
    /// Deferred updates is a lower level feature and was previously used internally to deal with
    /// update ordering issues in relation to scrollability and sizing. One can freely take
    /// advantage of it to do things like apply wrapping nodes on their own custom elements
    /// built on top of [`RawHaalkaEl`], but should be more wary of using it through
    /// [`.update_raw_el`](RawElWrapper::update_raw_el) on the provided higher level UI elements
    /// like [`El`](super::el::El) and [`Column`](super::column::Column).
    pub fn defer_update(
        mut self,
        append_direction: DeferredUpdaterAppendDirection,
        updater: impl FnOnce(RawHaalkaEl) -> RawHaalkaEl + Send + 'static,
    ) -> Self {
        match append_direction {
            DeferredUpdaterAppendDirection::Front => self.deferred_updaters.insert(0, Box::new(updater)),
            DeferredUpdaterAppendDirection::Back => self.deferred_updaters.push(Box::new(updater)),
        }
        self
    }

    /// Process the underlying [`NodeBuilder`] directly.
    pub fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder) -> NodeBuilder) -> Self {
        self.node_builder = Some(updater(self.node_builder.unwrap()));
        self
    }

    /// Consume this [`RawHaalkaEl`], running its deferred updaters and returning the underlying
    /// [`NodeBuilder`].
    pub fn into_node_builder(mut self) -> NodeBuilder {
        let deferred_updaters = self.deferred_updaters.drain(..).collect::<Vec<_>>();
        let mut self_ = self;
        for updater in deferred_updaters {
            self_ = updater(self_);
        }
        self_.node_builder.unwrap()
    }

    /// Run a function with mutable access to the [`World`] and this element's [`Entity`].
    pub fn on_spawn(self, on_spawn: impl FnOnce(&mut World, Entity) + Send + 'static) -> Self {
        self.update_node_builder(|node_builder| node_builder.on_spawn(on_spawn))
    }

    /// Run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`].
    pub fn on_spawn_with_system<T: IntoSystem<In<Entity>, (), Marker> + Send + 'static, Marker>(
        self,
        system: T,
    ) -> Self {
        self.on_spawn(|world, entity| {
            if let Err(error) = world.run_system_once_with(system, entity) {
                error!("failed to run system on spawn: {}", error);
            }
        })
    }

    /// Add a [`Bundle`] of components to this element.
    pub fn insert<B: Bundle>(self, bundle: B) -> Self {
        self.update_node_builder(|node_builder| node_builder.insert(bundle))
    }

    /// If the `forwarder` points to [`Some`] [`Entity`], Add a [`Bundle`] of components to that
    /// [`Entity`].
    pub fn insert_forwarded(
        self,
        forwarder: impl FnOnce(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        bundle: impl Bundle + 'static,
    ) -> Self {
        self.with_entity_forwarded(forwarder, move |mut entity| {
            entity.insert(bundle);
        })
    }

    /// Run a function with this element's [`EntityWorldMut`].
    pub fn with_entity(self, f: impl FnOnce(EntityWorldMut) + Send + 'static) -> Self {
        self.update_node_builder(|node_builder| node_builder.with_entity(f))
    }

    /// If the `forwarder` points to [`Some`] [`Entity`], run a function with that [`Entity`]'s
    /// [`EntityWorldMut`].
    pub fn with_entity_forwarded(
        self,
        forwarder: impl FnOnce(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        f: impl FnOnce(EntityWorldMut) + Send + 'static,
    ) -> Self {
        self.with_entity(move |mut entity| {
            if let Some(forwardee) = forwarder(&mut entity) {
                entity.world_scope(|world| {
                    if let Ok(forwardee) = world.get_entity_mut(forwardee) {
                        f(forwardee)
                    }
                })
            }
        })
    }

    /// Run a function with mutable access (via [`Mut`]) to this element's `C` [`Component`] if it
    /// exists.
    pub fn with_component<C: Component<Mutability = Mutable>>(self, f: impl FnOnce(Mut<C>) + Send + 'static) -> Self {
        self.with_entity(|mut entity| {
            if let Some(component) = entity.get_mut::<C>() {
                f(component);
            }
        })
    }

    /// If the `forwarder` points to [`Some`] [`Entity`], run a function with mutable access (via
    /// [`Mut`]) to that [`Entity`]'s `C` [`Component`] if it exists.
    pub fn with_component_forwarded<C: Component<Mutability = Mutable>>(
        self,
        forwarder: impl FnOnce(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        f: impl FnOnce(Mut<C>) + Send + 'static,
    ) -> Self {
        self.with_entity_forwarded(forwarder, move |mut entity| {
            if let Some(component) = entity.get_mut::<C>() {
                f(component)
            }
        })
    }

    /// Attach an [`Observer`] to this element.
    ///
    /// Attaches a special [`HaalkaObserver`] component to the entity, which allows it to be filtered by higher level tools (see [aalo](https://github.com/databasedav/aalo)).
    pub fn observe<E: Event, B: Bundle, Marker>(self, observer: impl IntoObserverSystem<E, B, Marker>) -> Self {
        self.on_spawn(|world, entity| {
            observe(world, entity, observer);
        })
    }

    /// Drop the [`Task`] when it completes or the entity is despawned.
    pub fn hold_tasks(self, tasks: impl IntoIterator<Item = Task<()>> + Send + 'static) -> Self {
        self.with_component::<TaskHolder>(|task_holder| {
            for task in tasks.into_iter() {
                task_holder.hold(task);
            }
        })
    }

    /// When this element is despawned, run a function with mutable access to the [`DeferredWorld`]
    /// and this element's [`Entity`].
    pub fn on_remove(self, on_remove: impl FnOnce(&mut DeferredWorld, Entity) + Send + Sync + 'static) -> Self {
        self.on_spawn(|world, entity| {
            if let Some(mut on_remove_component) = world.entity_mut(entity).get_mut::<OnRemove>() {
                on_remove_component.0.push(Box::new(on_remove));
            } else {
                world.entity_mut(entity).insert(OnRemove(vec![Box::new(on_remove)]));
            }
        })
    }

    /// Reactively run a [`Future`]-returning function with this element's [`Entity`] and the output
    /// of the [`Signal`].
    pub fn on_signal<T, Fut: Future<Output = ()> + Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        f: impl FnMut(Entity, T) -> Fut + Send + 'static,
    ) -> Self {
        self.update_node_builder(|node_builder| node_builder.on_signal(signal, f))
    }

    /// Reactively run a function with this element's [`Entity`] and the output of the [`Signal`].
    pub fn on_signal_sync<T>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Entity, T) + Send + 'static,
    ) -> Self {
        self.on_signal(signal, move |entity, value| {
            f(entity, value);
            async {}
        })
    }

    /// Reactively run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and
    /// the output of the [`Signal`].
    pub fn on_signal_with_system<T: Send + 'static, Marker>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        system: impl IntoSystem<In<(Entity, T)>, (), Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.on_spawn(clone!((system_holder) move |world, _| {
            let _ = system_holder.set(register_system(world, system));
        }))
        .on_signal(
            signal,
            clone!((system_holder) move |entity, input| {
                async_world().apply(run_system_with_entity(entity, system_holder.get().copied().unwrap(), input).handle_error_with(warn))
            }),
        )
        .apply(remove_system_holder_on_remove(system_holder))
    }

    /// Reactively run a [`System`], if the `forwarder` points to [`Some`] [`Entity`], which takes
    /// [`In`](`System::In`) that element's [`Entity`] and the output of the [`Signal`].
    #[allow(clippy::type_complexity)]
    pub fn on_signal_with_system_forwarded<T: Send + 'static, Marker1, Marker2>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        forwarder: impl IntoSystem<In<Entity>, Option<Entity>, Marker1> + Send + 'static,
        system: impl IntoSystem<In<(Entity, T)>, (), Marker2> + Send + 'static,
    ) -> Self {
        let forwarder_system_holder = Arc::new(OnceLock::new());
        let handler_system_holder = Arc::new(OnceLock::new());
        self.on_spawn(clone!((forwarder_system_holder, handler_system_holder) move |world, _| {
            let _ = forwarder_system_holder.set(register_system(world, forwarder));
            let _ = handler_system_holder.set(register_system(world, system));
        }))
        .on_signal_with_system(
            signal,
            clone!((forwarder_system_holder, handler_system_holder) move |In((entity, input)): In<(Entity, T)>, mut commands: Commands| {
                commands.queue(clone!((forwarder_system_holder, handler_system_holder) move |world: &mut World| {
                    if let Ok(Some(forwardee)) = world.run_system_with(forwarder_system_holder.get().copied().unwrap(), entity) {
                        let _ = world.run_system_with(handler_system_holder.get().copied().unwrap(), (forwardee, input));
                    }
                }))
            }),
        )
        .apply(remove_system_holder_on_remove(forwarder_system_holder))
        .apply(remove_system_holder_on_remove(handler_system_holder))
    }

    /// Reactively run a function with this element's [`EntityWorldMut`] and the output of the
    /// [`Signal`].
    pub fn on_signal_with_entity<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(EntityWorldMut, T) + Send + Sync + 'static,
    ) -> Self {
        self.on_signal_with_system(
            signal,
            move |In((entity, value)): In<(Entity, T)>, world: &mut World| {
                if let Ok(entity) = world.get_entity_mut(entity) {
                    f(entity, value)
                }
            },
        )
    }

    /// Reactively run a function, if the `forwarder` points to [`Some`] [`Entity`],
    /// with that [`Entity`]'s [`EntityWorldMut`] and the output of the [`Signal`].
    pub fn on_signal_with_entity_forwarded<T: Send + 'static, Marker>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        forwarder: impl IntoSystem<In<Entity>, Option<Entity>, Marker> + Send + 'static,
        mut f: impl FnMut(EntityWorldMut, T) + Send + Sync + 'static,
    ) -> Self {
        self.on_signal_with_system_forwarded(
            signal,
            forwarder,
            move |In((entity, value)): In<(Entity, T)>, world: &mut World| {
                if let Ok(entity) = world.get_entity_mut(entity) {
                    f(entity, value)
                }
            },
        )
    }

    /// Reactively run a function with mutable access (via [`Mut`]) to this element's `C`
    /// [`Component`] if it exists and the output of the [`Signal`].
    pub fn on_signal_with_component<T: Send + 'static, C: Component<Mutability = Mutable>>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Mut<C>, T) + Send + Sync + 'static,
    ) -> Self {
        self.on_signal_with_system(
            signal,
            move |In((entity, value)): In<(Entity, T)>, mut query: Query<&mut C>| {
                if let Ok(component) = query.get_mut(entity) {
                    f(component, value)
                }
            },
        )
    }

    /// Reactively run a function, if the `forwarder` points to [`Some`] [`Entity`], with mutable
    /// access (via [`Mut`]) to that [`Entity`]'s `C` [`Component`] if it exists.
    pub fn on_signal_with_component_forwarded<T: Send + 'static, C: Component<Mutability = Mutable>, Marker>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        forwarder: impl IntoSystem<In<Entity>, Option<Entity>, Marker> + Send + 'static,
        mut f: impl FnMut(Mut<C>, T) + Send + Sync + 'static,
    ) -> Self {
        self.on_signal_with_system_forwarded(
            signal,
            forwarder,
            move |In((entity, value)): In<(Entity, T)>, mut query: Query<&mut C>| {
                if let Ok(component) = query.get_mut(entity) {
                    f(component, value)
                }
            },
        )
    }

    /// Reactively set this element's `C` [`Component`]. If the [`Signal`] outputs [`None`], the `C`
    /// [`Component`] is removed.
    pub fn component_signal<C: Component, S: Signal<Item = impl Into<Option<C>>> + Send + 'static>(
        mut self,
        component_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(component_option_signal) = component_option_signal_option.into() {
            // TODO: need partial_eq derivations for all the node related components to minimize updates
            // with .dedupe
            self = self.on_signal_with_entity::<Option<C>>(
                component_option_signal.map(|into_component_option| into_component_option.into()),
                move |mut entity, component_option| {
                    if let Some(component) = component_option {
                        entity.insert(component);
                    } else {
                        entity.remove::<C>();
                    }
                },
            );
        }
        self
    }

    /// Reactively set the `C` [`Component`] of the [`Entity`] that the `forwarder` points to if it
    /// points to [`Some`] [`Entity`]. If the [`Signal`] outputs [`None`], the `C` [`Component`] is
    /// removed.
    pub fn component_signal_forwarded<C: Component, Marker>(
        self,
        forwarder: impl IntoSystem<In<Entity>, Option<Entity>, Marker> + Send + 'static,
        component_option_signal: impl Signal<Item = impl Into<Option<C>>> + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity_forwarded(
            component_option_signal.map(|into_component_option| into_component_option.into()),
            forwarder,
            move |mut entity, component_option| {
                if let Some(component) = component_option {
                    entity.insert(component);
                } else {
                    entity.remove::<C>();
                }
            },
        )
    }

    /// Reactively send an [`Event`] based on this element's [`Entity`] and the output of the
    /// [`Signal`].
    pub fn on_signal_send_event<T, E: Event>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Entity, T) -> E + Send + 'static,
    ) -> Self {
        self.on_signal(signal, move |entity, value| async_world().send_event(f(entity, value)))
    }

    /// When this element receives an `E` [`Event`] and does not have a `Disabled`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and the [`Event`]; if the element has a `PropagationStopped` [`Component`], the
    /// event will not bubble up the hierarchy. If propagation is conditional on logic within the
    /// body of the `handler`, use [.observe](`Self::observe`) instead to access the mutable
    /// [`Trigger<E>`] directly.
    pub fn on_event_with_system_disableable_propagation_stoppable<
        E: Event + Clone,
        Disabled: Component,
        PropagationStopped: Component,
        Marker,
    >(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self
            .on_spawn(clone!((system_holder) move |world, entity| {
                let handler = register_system(world, handler);
                let _ = system_holder.set(handler);
                observe(world, entity, move |mut event: Trigger<E>, disabled: Query<&Disabled>, propagation_stopped: Query<&PropagationStopped>, mut commands: Commands| {
                    if !disabled.contains(entity) {
                        commands.run_system_with(handler, (entity, (*event).clone()));
                        if propagation_stopped.contains(entity) {
                            event.propagate(false);
                        }
                    }
                });
            }))
            .apply(remove_system_holder_on_remove(system_holder))
    }

    /// When this element receives an `E` [`Event`], run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and the [`Event`].
    pub fn on_event_with_system<E: Event + Clone, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.on_event_with_system_disableable_propagation_stoppable::<E, EventHandlingDisabled<E>, EventPropagationStopped<E>, _>(
            handler,
        )
    }

    /// When this element receives an `E` [`Event`] and does not have a `Disabled`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`] and the [`Event`].
    pub fn on_event_with_system_disableable<E: Event + Clone, Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.on_event_with_system_disableable_propagation_stoppable::<E, Disabled, EventPropagationStopped<E>, _>(
            handler,
        )
    }

    /// When this element receives an `E` [`Event`], run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and the [`Event`], reactively
    /// controlling whether this handling is disabled with a [`Signal`]. Critically
    /// note that this disabling is not frame perfect, e.g. one should not expect the handler to
    /// be disabled the same frame that the [`Signal`] outputs `true`. If one needs frame
    /// perfect disabling, use
    /// [`.on_event_with_system_disableable`](Self::on_event_with_system_disableable).
    pub fn on_event_with_system_disableable_signal<E: Event + Clone, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
        disabled: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.component_signal::<EventHandlingDisabled<E>, _>(disabled.map_true(|| EventHandlingDisabled(PhantomData)))
            .on_event_with_system_disableable::<E, EventHandlingDisabled<E>, _>(handler)
    }

    /// When this element receives an `E` [`Event`], run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and the [`Event`]; if the element has a
    /// `PropagationStopped` [`Component`], the event will not bubble up the hierarchy. If
    /// propagation is conditional on logic within the body of the `handler`, use
    /// [.observe](`Self::observe`) instead to access the mutable [`Trigger<E>`] directly.
    pub fn on_event_with_system_propagation_stoppable<E: Event + Clone, PropagationStopped: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.on_event_with_system_disableable_propagation_stoppable::<E, EventHandlingDisabled<E>, PropagationStopped, _>(
            handler,
        )
    }

    /// When this element receives an `E` [`Event`], run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and the [`Event`], reactively
    /// controlling whether this handling is disabled with a [`Signal`]. Critically
    /// note that this propagation stopping is not frame perfect, e.g. one should not expect the
    /// handler to stop propagation the same frame that the [`Signal`] outputs `true`. If one
    /// needs frame perfect propagation stopping, use
    /// [`.on_event_with_system_propagation_stoppable`](Self::on_event_with_system_propagation_stoppable).
    /// If propagation is conditional on logic within the body of the `handler`, use
    /// [.observe](`Self::observe`) instead to access the mutable [`Trigger<E>`] directly.
    pub fn on_event_with_system_propagation_stoppable_signal<E: Event + Clone, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.component_signal::<EventPropagationStopped<E>, _>(
            propagation_stopped.map_true(|| EventPropagationStopped(PhantomData)),
        )
        .on_event_with_system_propagation_stoppable::<E, EventPropagationStopped<E>, _>(handler)
    }

    /// When this element receives an `E` [`Event`], run a function with the [`Event`],
    /// stopping the event from bubbling up the hierarchy.
    pub fn on_event_with_system_stop_propagation<E: Event + Clone, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.insert(EventPropagationStopped::<E>(PhantomData))
            .on_event_with_system_propagation_stoppable::<E, EventPropagationStopped<E>, _>(handler)
    }

    /// When this element receives an `E` [`Event`], run a function with the [`Event`].
    pub fn on_event<E: Event + Clone>(self, mut handler: impl FnMut(E) + Send + Sync + 'static) -> Self {
        self.on_event_with_system::<E, _>(move |In((_, event))| handler(event))
    }

    /// When this element receives an `E` [`Event`] and does not have a `Disabled`
    /// [`Component`], run a function with the [`Event`].
    pub fn on_event_disableable<E: Event + Clone, Disabled: Component>(
        self,
        mut handler: impl FnMut(E) + Send + Sync + 'static,
    ) -> Self {
        self.on_event_with_system_disableable::<E, Disabled, _>(move |In((_, event))| handler(event))
    }

    /// When this element receives an `E` [`Event`], run a with the [`Event`],
    /// reactively controlling whether this handling is disabled with a [`Signal`].
    /// Critically note that this disabling is not frame perfect, e.g. one should not expect the
    /// handler to be disabled the same frame that the [`Signal`] outputs `true`. If one needs
    /// frame perfect disabling, use
    /// [`.on_event_disableable`](Self::on_event_disableable)
    pub fn on_event_disableable_signal<E: Event + Clone>(
        self,
        mut handler: impl FnMut(E) + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.on_event_with_system_disableable_signal::<E, _>(move |In((_, event))| handler(event), disabled)
    }

    /// When this element receives an `E` [`Event`], run a function with the [`Event`];
    /// if the element has a `PropagationStopped` [`Component`], the event will not bubble up
    /// the hierarchy. If propagation is conditional on logic within the body of the `handler`,
    /// use [.observe](`Self::observe`) instead to access the mutable [`Trigger<E>`] directly.
    pub fn on_event_propagation_stoppable<E: Event + Clone, Marker, PropagationStopped: Component>(
        self,
        mut handler: impl FnMut(E) + Send + Sync + 'static,
    ) -> Self {
        self.on_event_with_system_propagation_stoppable::<E, PropagationStopped, _>(move |In((_, event))| {
            handler(event)
        })
    }

    /// When this element receives an `E` [`Event`], run a function with the [`Event`],
    /// reactively controlling whether the event bubbles up the hierarchy with a [`Signal`].
    /// If propagation is conditional on logic within the  body of the `handler`, use
    /// [.observe](`Self::observe`) instead to access the mutable [`Trigger<E>`] directly.
    pub fn on_event_propagation_stoppable_signal<E: Event + Clone>(
        self,
        mut handler: impl FnMut(E) + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.on_event_with_system_propagation_stoppable_signal(
            move |In((_, event))| handler(event),
            propagation_stopped,
        )
    }

    /// When this element receives an `E` [`Event`], run a function with the [`Event`],
    /// stopping the event from bubbling up the hierarchy.
    pub fn on_event_stop_propagation<E: Event + Clone>(
        self,
        mut handler: impl FnMut(E) + Send + Sync + 'static,
    ) -> Self {
        self.on_event_with_system_stop_propagation::<E, _>(move |In((_, event))| handler(event))
    }

    /// When this element receives an `E` [`Event`], run a function run a function with the
    /// [`Event`], reactively controlling whether the event bubbles up the hierarchy and
    /// reactively disabling this handling. Critically note that this disabling and propagation
    /// stopping is not frame perfect, e.g. one should not expect the handler to be disabled or
    /// stop propagation the same frame that the respective [`Signal`] outputs `true`. If one needs
    /// frame perfect disabling and propagation stopping, use
    /// [`.on_event_with_system_disableable_propagation_stoppable`](Self::on_event_with_system_disableable_propagation_stoppable).
    /// If propagation is conditional on logic within the body of the `handler`, use
    /// [.observe](`Self::observe`) instead to access the mutable [`Trigger<E>`] directly.
    pub fn on_event_disableable_propagation_stoppable_signal<E: Event + Clone>(
        self,
        mut handler: impl FnMut(E) + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + Send + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self
        .component_signal::<EventHandlingDisabled<E>, _>(disabled.map_true(|| EventHandlingDisabled(PhantomData)))
        .component_signal::<EventPropagationStopped<E>, _>(propagation_stopped.map_true(|| EventPropagationStopped(PhantomData)))
        .on_event_with_system_disableable_propagation_stoppable::<E, EventHandlingDisabled<E>, EventPropagationStopped<E>, _>(
            move |In((_, event))| handler(event),
        )
    }

    /// Declare a static child.
    pub fn child<IORE: IntoOptionRawElement>(self, child_option: IORE) -> Self {
        if let Some(child) = child_option.into_option_element() {
            return self.update_node_builder(|node_builder| node_builder.child(child.into_raw().into_node_builder()));
        }
        self
    }

    /// Declare a reactive child. When the [`Signal`] outputs [`None`], the child is removed.
    pub fn child_signal<IORE: IntoOptionRawElement>(
        self,
        child_option_signal: impl Signal<Item = IORE> + Send + 'static,
    ) -> Self {
        self.update_node_builder(|node_builder| {
            node_builder.child_signal(child_option_signal.map(|child_option| {
                child_option
                    .into_option_element()
                    .map(|child| child.into_raw().into_node_builder())
            }))
        })
    }

    /// Declare static children.
    pub fn children<IORE: IntoOptionRawElement, I: IntoIterator<Item = IORE>>(self, children_options: I) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        self.update_node_builder(|node_builder| {
            node_builder.children(
                children_options
                    .into_iter()
                    .filter_map(|child_option| child_option.into_option_element())
                    .map(|child| child.into_raw().into_node_builder()),
            )
        })
    }

    /// Declare reactive children.
    pub fn children_signal_vec<IORE: IntoOptionRawElement>(
        self,
        children_options_signal_vec: impl SignalVec<Item = IORE> + Send + 'static,
    ) -> Self {
        self.update_node_builder(|node_builder| {
            node_builder.children_signal_vec(
                children_options_signal_vec
                    .filter_map(|child_option| child_option.into_option_element())
                    .map(|child| child.into_raw().into_node_builder()),
            )
        })
    }
}

fn run_system_with_entity<I: Send + 'static>(
    entity: Entity,
    id: SystemId<In<(Entity, I)>>,
    input: I,
) -> impl Command<Result> {
    move |world: &mut World| -> Result {
        if world.get_entity(entity).is_ok() {
            world.run_system_with(id, (entity, input))?;
        }
        Ok(())
    }
}

fn on_remove_on_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let fs = world
        .get_mut::<OnRemove>(entity)
        .unwrap()
        .0
        .drain(..)
        .collect::<Vec<_>>();
    for f in fs {
        f(&mut world, entity);
    }
}

#[allow(clippy::type_complexity)]
#[derive(Component)]
#[component(on_remove = on_remove_on_remove)]
struct OnRemove(Vec<Box<dyn FnOnce(&mut DeferredWorld, Entity) + Send + Sync + 'static>>);

/// Marker [`Component`] for filtering `SystemId` `Entity`s managed by haalka.
#[derive(Component)]
pub struct HaalkaOneShotSystem;

pub(crate) fn register_system<I: SystemInput + 'static, O: 'static, Marker, S: IntoSystem<I, O, Marker> + 'static>(
    world: &mut World,
    system: S,
) -> SystemId<I, O> {
    let system = world.register_system(system);
    if let Ok(mut entity) = world.get_entity_mut(system.entity()) {
        entity.insert(HaalkaOneShotSystem);
    }
    system
}

/// Marker [`Component`] for filtering `Observer` `Entity`s managed by haalka.
#[derive(Component)]
pub struct HaalkaObserver;

pub(crate) fn observe<E: Event, B: Bundle, Marker>(
    world: &mut World,
    entity: Entity,
    observer: impl IntoObserverSystem<E, B, Marker>,
) -> EntityWorldMut<'_> {
    world.spawn((Observer::new(observer).with_entity(entity), HaalkaObserver))
}

#[derive(Component)]
struct EventHandlingDisabled<E: Event>(PhantomData<E>);

#[derive(Component)]
struct EventPropagationStopped<E: Event>(PhantomData<E>);

/// Thin wrapper trait around [`RawHaalkaEl`] to allow consumers to target custom types when
/// composing [`RawHaalkaEl`]s.
pub trait RawElement: Sized {
    /// Convert this type into a [`RawHaalkaEl`].
    fn into_raw(self) -> RawHaalkaEl;
}

impl<REW: RawElWrapper> RawElement for REW {
    fn into_raw(self) -> RawHaalkaEl {
        self.into_raw_el()
    }
}

// TODO: why is this a conflicting impl ?
// impl<B: Bundle> RawElement for B {
//     fn into_raw(self) -> RawHaalkaEl {
//         RawHaalkaEl::from(self)
//     }
// }

/// Allows consumers to pass non-[`RawElWrapper`] types to the child methods of [`RawHaalkaEl`].
pub trait IntoRawElement {
    /// The type of the [`RawElement`] that this type is converted into
    type EL: RawElement;
    /// Convert this type into an [`RawElement`].
    fn into_raw_element(self) -> Self::EL;
}

impl<T: RawElement> IntoRawElement for T {
    type EL = T;
    fn into_raw_element(self) -> Self::EL {
        self
    }
}

/// Thin wrapper trait around [`RawElement`] that allows consumers to pass [`Option`]s to the child
/// methods of [`RawHaalkaEl`].
pub trait IntoOptionRawElement {
    /// The type of the [`RawElement`] that this type is maybe converted into.
    type EL: RawElement;
    /// Maybe convert this type into a [`RawElement`].
    fn into_option_element(self) -> Option<Self::EL>;
}

impl<E: RawElement, IE: IntoRawElement<EL = E>> IntoOptionRawElement for Option<IE> {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        self.map(|into_element| into_element.into_raw_element())
    }
}

impl<E: RawElement, IE: IntoRawElement<EL = E>> IntoOptionRawElement for IE {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        Some(self.into_raw_element())
    }
}

// TODO: proc macro for RawElWrapper which scans through the fields of a struct and implements the
// trait for whatever RawHaalkaEl is found first
/// [`RawElWrapper`]s can be passed to the child methods of [`RawHaalkaEl`]. This can be used to
/// create custom non-UI "widgets". See [`ElementWrapper`](super::element::ElementWrapper) for what
/// this looks like in a UI context.
pub trait RawElWrapper: Sized {
    /// Mutable reference to the [`RawHaalkaEl`] that this wrapper wraps.
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl;

    /// Process the wrapped [`RawHaalkaEl`] directly.
    fn update_raw_el(mut self, updater: impl FnOnce(RawHaalkaEl) -> RawHaalkaEl) -> Self {
        let raw_el = mem::replace(self.raw_el_mut(), RawHaalkaEl::new_dummy());
        *self.raw_el_mut() = updater(raw_el);
        self
    }

    /// Consume this wrapper, returning the wrapped [`RawHaalkaEl`].
    fn into_raw_el(mut self) -> RawHaalkaEl {
        mem::replace(self.raw_el_mut(), RawHaalkaEl::new_dummy())
    }
}

/// Required to allow passing [`RawHaalkaEl`]s to [`RawHaalkaEl`]'s `.child` methods.
impl RawElement for RawHaalkaEl {
    fn into_raw(self) -> RawHaalkaEl {
        self
    }
}

/// Allows [`RawElement`]s and their [wrappers](RawElWrapper) to be spawned into the world.
pub trait Spawnable: RawElement
where
    Self: Sized,
{
    /// Spawn the element into the world.
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw().into_node_builder().spawn(world)
    }
}

impl<REW: RawElement> Spawnable for REW {}

#[allow(missing_docs)]
pub mod utils {
    use super::*;

    /// If [`Some`] [`System`] is returned by the `getter`, remove it
    /// from the [`World`] on element removal.
    pub fn remove_system_on_remove<I: SystemInput + 'static, O: 'static>(
        getter: impl FnOnce() -> Option<SystemId<I, O>> + Send + Sync + 'static,
    ) -> impl FnOnce(RawHaalkaEl) -> RawHaalkaEl {
        |raw_el| {
            raw_el.on_remove(move |world, _| {
                if let Some(system) = getter() {
                    world.commands().queue(move |world: &mut World| {
                        let _ = world.unregister_system(system);
                    })
                }
            })
        }
    }

    /// Remove the held system from the [`World`] on element removal.
    pub fn remove_system_holder_on_remove<I: SystemInput + 'static, O: 'static>(
        system_holder: Arc<OnceLock<SystemId<I, O>>>,
    ) -> impl FnOnce(RawHaalkaEl) -> RawHaalkaEl {
        remove_system_on_remove(move || system_holder.get().copied())
    }

    /// Run an element's deferred updaters without spawning.
    pub fn flush_deferred_updaters<T: RawElement>(raw_el: T) -> RawHaalkaEl {
        raw_el.into_raw().into_node_builder().apply(RawHaalkaEl::from)
    }
}
