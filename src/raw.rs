use std::{
    mem,
    sync::{Arc, Mutex},
};

use async_lock;
use bevy::{prelude::*, tasks::Task};
use bevy_eventlistener::prelude::*;
use enclose::enclose as clone;
use futures_signals::{
    signal::{always, Mutable, Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};
use haalka_futures_signals_ext::{Future, SignalExtExt};

use super::{
    node_builder::{async_world, NodeBuilder, TaskHolder},
    utils::spawn,
};

/// [haalka](crate)'s core abstraction, allowing one to rig any [`Entity`] with ergonomic
/// [`futures_signals::Signal`](https://docs.rs/futures-signals/latest/futures_signals/signal/trait.Signal.html) driven reactivity, including
/// methods for registering children, [`Component`]s, [event listeners](On), and [`System`]s all
/// using a declarative builder pattern/[fluent interface](https://en.wikipedia.org/wiki/Fluent_interface).
/// Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon/tree/main)'s [`raw_el`](https://github.com/MoonZoon/MoonZoon/tree/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/raw_el).
pub struct RawHaalkaEl {
    node_builder: Option<NodeBuilder>,
    deferred_updaters: Vec<Box<dyn FnOnce(RawHaalkaEl) -> RawHaalkaEl + Send + 'static>>,
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

pub enum AppendDirection {
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
    /// Useful when updates must be applied after some node is wrapped with another, for example,
    /// the [`super::Sizeable`] methods defer their updates to the back of the line because they
    /// must be applied after any [`super::Scrollable`] wrapping container is applied, whose
    /// application is also deferred, but to the *front* of the back of the line.
    ///
    /// # Notes
    /// Deferred updates is a lower level feature and is used internally to deal with update
    /// ordering issues in relation to scrollability and sizing. One can freely take advantage of it
    /// to do things like apply wrapping nodes on their own custom elements built on top of
    /// [`RawHaalkaEl`], but should be more wary of using it through
    /// [`.update_raw_el`](RawElWrapper::update_raw_el) on the provided higher level UI elements
    /// like [`super::El`] and [`super::Column`].
    pub fn defer_update(
        mut self,
        append_direction: AppendDirection,
        updater: impl FnOnce(RawHaalkaEl) -> RawHaalkaEl + Send + 'static,
    ) -> Self {
        match append_direction {
            AppendDirection::Front => self.deferred_updaters.insert(0, Box::new(updater)),
            AppendDirection::Back => self.deferred_updaters.push(Box::new(updater)),
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

    /// Add a [`Bundle`] of components to this element.
    pub fn insert<B: Bundle>(self, bundle: B) -> Self {
        self.update_node_builder(|node_builder| node_builder.insert(bundle))
    }

    /// If the `forwarder` points to [`Some`] [`Entity`], Add a [`Bundle`] of components to that
    /// [`Entity`].
    pub fn insert_forwarded(
        self,
        forwarder: impl FnOnce(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        bundle: impl Bundle + Send + 'static,
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
                    if let Some(forwardee) = world.get_entity_mut(forwardee) {
                        f(forwardee)
                    }
                })
            }
        })
    }

    /// Run a function with mutable access (via [`Mut`]) to this element's `C` [`Component`] if it
    /// exists.
    pub fn with_component<C: Component>(self, f: impl FnOnce(Mut<C>) + Send + 'static) -> Self {
        self.with_entity(|mut entity| {
            if let Some(component) = entity.get_mut::<C>() {
                f(component);
            }
        })
    }

    /// If the `forwarder` points to [`Some`] [`Entity`], run a function with mutable access (via
    /// [`Mut`]) to that [`Entity`]'s `C` [`Component`] if it exists.
    pub fn with_component_forwarded<C: Component>(
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

    /// Drop the [`Task`]s when the element is despawned.
    pub fn hold_tasks(self, tasks: impl IntoIterator<Item = Task<()>> + Send + 'static) -> Self {
        self.with_component::<TaskHolder>(|mut task_holder| {
            for task in tasks.into_iter() {
                task_holder.hold(task);
            }
        })
    }

    // TODO: requires bevy 0.14
    // pub fn on_remove(self, on_remove: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) ->
    // Self { self.on_spawn(|world, entity| {
    //         if let Some(mut on_remove_component) = world.entity_mut(entity).get_mut::<OnRemove>() {
    //             on_remove_component.0.push(Box::new(on_remove));
    //         } else {
    //             world.entity_mut(entity).insert(OnRemove(vec![Box::new(on_remove)]));
    //         }
    //     })
    // }

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

    /// Reactively run a function with this element's [`EntityWorldMut`] and the output of the
    /// [`Signal`].
    pub fn on_signal_with_entity<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        f: impl FnMut(EntityWorldMut, T) + Send + 'static,
    ) -> Self {
        let f = Arc::new(Mutex::new(f));
        self.on_signal(signal, move |entity, value| {
            async_world().apply(clone!((f) move |world: &mut World| {
                if let Some(entity) = world.get_entity_mut(entity) {
                    f.lock().unwrap()(entity, value)
                }
            }))
        })
    }

    /// Reactively run a function with that [`Entity`]'s [`EntityWorldMut`] and the output of the
    /// [`Signal`].
    pub fn on_signal_with_entity_forwarded<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut forwarder: impl FnMut(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        mut f: impl FnMut(EntityWorldMut, T) + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity(signal, move |mut entity, value| {
            if let Some(forwardee) = forwarder(&mut entity) {
                entity.world_scope(|world| {
                    if let Some(forwardee) = world.get_entity_mut(forwardee) {
                        f(forwardee, value);
                    }
                })
            }
        })
    }

    /// Reactively run a function with mutable access (via [`Mut`]) to this element's `C`
    /// [`Component`] if it exists and the output of the [`Signal`].
    pub fn on_signal_with_component<T: Send + 'static, C: Component>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Mut<C>, T) + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity(signal, move |mut entity, value| {
            if let Some(component) = entity.get_mut::<C>() {
                f(component, value)
            }
        })
    }

    /// Reactively run a function, if the `forwarder` points to [`Some`] [`Entity`], with mutable
    /// access (via [`Mut`]) to that [`Entity`]'s `C` [`Component`] if it exists.
    pub fn on_signal_with_component_forwarded<T: Send + 'static, C: Component>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        forwarder: impl FnMut(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        mut f: impl FnMut(Mut<C>, T) + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity_forwarded(signal, forwarder, move |mut entity, value| {
            if let Some(component) = entity.get_mut::<C>() {
                f(component, value)
            }
        })
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
    pub fn component_signal_forwarded<C: Component>(
        self,
        forwarder: impl FnMut(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
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
        mut event_f: impl FnMut(Entity, T) -> E + Send + 'static,
    ) -> Self {
        self.on_signal(signal, move |entity, value| {
            async_world().send_event(event_f(entity, value))
        })
    }

    /// Reactively run an IO [`System`] that takes [`In`](`System::In`) this node's [`Entity`] and
    /// the output of the [`Signal`] and then run a [`Future`]-returning function with this
    /// element's [`Entity`] and the [`Out`](`System::Out`)put of the [`System`].
    pub fn on_signal_one_shot_io<I: Send + 'static, O: Send + 'static, M, Fut: Future<Output = ()> + Send + 'static>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), O, M> + Send + 'static,
        f: impl FnMut(Entity, O) -> Fut + Send + 'static,
    ) -> Self {
        let system_holder = Mutable::new(None);
        let f = Arc::new(async_lock::Mutex::new(f));
        self.hold_tasks([spawn(clone!((system_holder) async move {
            system_holder.set(Some(async_world().register_io_system(system).await));
        }))])
        // .on_remove(move |world, _| {
        //     if let Some(system) = system_holder.take() {
        //         // TODO: https://github.com/dlom/bevy-async-ecs/issues/5#issuecomment-2119180363
        //         world.remove_system(system.id)
        //     }
        // })
        .on_signal(signal, move |entity, input| {
            clone!((system_holder, f) async move {
                if system_holder.lock_ref().is_none() {
                    system_holder.signal_ref(Option::is_some).wait_for(true).await;
                }
                let output = system_holder.get_cloned().unwrap().run((entity, input)).await;
                // need async mutex because sync mutex guards are not `Send`
                f.lock().await(entity, output).await;
            })
        })
    }

    /// Reactively run an IO [`System`] that takes [`In`](`System::In`) this node's [`Entity`] and
    /// the output of the [`Signal`] and then run a function with this element's [`EntityWorldMut`]
    /// and the [`Out`](`System::Out`)put of the [`System`].
    pub fn on_signal_one_shot_io_with_entity<I: Send + 'static, O: Send + 'static, M>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), O, M> + Send + 'static,
        f: impl FnMut(EntityWorldMut, O) + Send + 'static,
    ) -> Self {
        let f = Arc::new(Mutex::new(f));
        self.on_signal_one_shot_io(signal, system, move |entity, value| {
            async_world().apply(clone!((f) move |world: &mut World| {
                if let Some(entity) = world.get_entity_mut(entity) {
                    f.lock().unwrap()(entity, value);
                }
            }))
        })
    }

    /// Reactively run an IO [`System`] that takes [`In`](`System::In`) this node's [`Entity`] and
    /// the output of the [`Signal`] and then run a function with mutable access (via [`Mut`]) to
    /// this element's `C` [`Component`] if it exists and the [`Out`](`System::Out`)put of the
    /// [`System`].
    pub fn on_signal_one_shot_io_with_component<I: Send + 'static, O: Send + 'static, M, C: Component>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), O, M> + Send + 'static,
        mut f: impl FnMut(Mut<C>, O) + Send + 'static,
    ) -> Self {
        self.on_signal_one_shot_io_with_entity(signal, system, move |mut entity, value| {
            if let Some(component) = entity.get_mut::<C>() {
                f(component, value);
            }
        })
    }

    /// Reactively run an IO [`System`] that takes [`In`](`System::In`) this node's [`Entity`] and
    /// the output of the [`Signal`].
    pub fn on_signal_one_shot<I: Send + 'static, M>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), (), M> + Send + 'static,
    ) -> Self {
        self.on_signal_one_shot_io(signal, system, |_, _| async {})
    }

    /// Reactively run an IO [`System`] that takes [`In`](`System::In`) this node's [`Entity`] and
    /// the output of the [`Signal`] and then set this element's `C` [`Component`] to the
    /// [`Out`](`System::Out`)put of the [`System`].
    pub fn component_one_shot_signal<I: Send + 'static, M, C: Component, IOC: Into<Option<C>> + Send + 'static>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), IOC, M> + Send + 'static,
    ) -> Self {
        self.on_signal_one_shot_io_with_entity(signal, system, |mut entity, into_option_component| {
            if let Some(component) = into_option_component.into() {
                entity.insert(component);
            } else {
                entity.remove::<C>();
            }
        })
    }

    /// Run a [`System`] when this element receives an `E` [`EntityEvent`].
    pub fn on_event_with_system<E: EntityEvent, Marker>(self, handler: impl IntoSystem<(), (), Marker>) -> Self {
        self.insert(On::<E>::run(handler))
    }

    /// Run a [`System`] when this element receives an `E` [`EntityEvent`], reactively disabling
    /// this handling.
    pub fn on_event_with_system_disableable<E: EntityEvent, Marker>(
        self,
        handler: impl IntoSystem<(), (), Marker> + Send + 'static,
        disabled: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        let handler_holder = Mutable::new(Some(On::<E>::run(handler)));
        self.on_signal_with_entity(disabled.dedupe(), move |mut entity, disabled| {
            if disabled {
                handler_holder.set(entity.take::<On<E>>());
            } else {
                entity.insert(handler_holder.lock_mut().take().unwrap());
            }
        })
    }

    /// When this element receives an `E` [`EntityEvent`], run a function with access to the event's
    /// data.
    pub fn on_event<E: EntityEvent>(self, mut handler: impl FnMut(Listener<E>) + Send + Sync + 'static) -> Self {
        self.on_event_with_system::<E, _>(move |event: Listener<E>| handler(event))
    }

    /// When this element receives an `E` [`EntityEvent`], run a function with mutable access to the
    /// event's data.
    pub fn on_event_mut<E: EntityEvent>(self, mut handler: impl FnMut(ListenerMut<E>) + Send + Sync + 'static) -> Self {
        self.on_event_with_system::<E, _>(move |event: ListenerMut<E>| handler(event))
    }

    /// When this element receives an `E` [`EntityEvent`], run a function with access to the event's
    /// data, reactively controlling whether the event bubbles up the hierarchy.
    pub fn on_event_propagation_stoppable<E: EntityEvent>(
        self,
        mut handler: impl FnMut(&E) + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        let propagation_stopped_mutable = Mutable::new(false);
        let syncer = spawn(propagation_stopped
            .for_each_sync(clone!((propagation_stopped_mutable) move |propagation_stopped| propagation_stopped_mutable.set_neq(propagation_stopped))));
        self.hold_tasks([syncer]).on_event_mut::<E>(move |mut event| {
            if propagation_stopped_mutable.get() {
                event.stop_propagation();
            }
            handler(&**event);
        })
    }

    /// When this element receives an `E` [`EntityEvent`], run a function with access to the event's
    /// data, stopping the event from bubbling up the hierarchy.
    pub fn on_event_stop_propagation<E: EntityEvent>(self, handler: impl FnMut(&E) + Send + Sync + 'static) -> Self {
        self.on_event_propagation_stoppable::<E>(handler, always(true))
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

// struct OnRemove(Vec<Box<dyn FnOnce(&mut World, Entity) + Send + Sync + 'static>>);

// TODO: requires bevy 0.14
// impl Component for OnRemove {
//     const STORAGE_TYPE: StorageType = StorageType::Table;

//     fn register_component_hooks(hooks: &mut ComponentHooks) {
//         hooks.on_remove.(|mut world, entity, component_id| {
//             for f in world.get_mut::<OnRemove>(entity).unwrap().0.drain(..) {
//                 f(&mut world, entity);
//             }
//         })
//     }
// }

/// Thin wrapper trait around [`RawHaalkaEl`] to allow consumers to target custom types when
/// composing [`RawHaalkaEl`]s.
pub trait RawElement: Sized {
    /// Convert this type into a [`RawHaalkaEl`].
    fn into_raw(self) -> RawHaalkaEl;
}

impl<REW: RawElWrapper> RawElement for REW {
    fn into_raw(self) -> RawHaalkaEl {
        self.into_raw_el().into()
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

// TODO: derive macro for RawElWrapper which scans through the fields of a struct and implements the
// trait for whatever RawHaalkaEl is found first
/// [`RawElWrapper`]s can be passed to the child methods of [`RawHaalkaEl`]. This can be used to
/// create custom non-UI "widgets". See [`super::ElementWrapper`] for what this looks like in a UI
/// context.
pub trait RawElWrapper: Sized {
    /// Mutable reference to the [`RawHaalkaEl`] that this wrapper wraps.
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl;

    /// Process the wrapped [`RawHaalkaEl`] directly.
    fn update_raw_el(mut self, updater: impl FnOnce(RawHaalkaEl) -> RawHaalkaEl) -> Self {
        let raw_el = mem::replace(self.raw_el_mut(), RawHaalkaEl::new_dummy());
        mem::swap(self.raw_el_mut(), &mut updater(raw_el));
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

/// Allows [`RawElement`]s and their wrappers to be spawned into the world.
pub trait Spawnable: RawElement {
    /// Spawn the element into the world.
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw().into_node_builder().spawn(world)
    }
}

impl<REW: RawElement> Spawnable for REW {}
