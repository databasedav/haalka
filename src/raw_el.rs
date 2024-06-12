use std::{
    mem,
    sync::{Arc, Mutex},
};

use async_lock;
use bevy::{prelude::*, tasks::Task};
use bevy_mod_picking::prelude::*;
use enclose::enclose as clone;
use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};
use futures_signals_ext::*;
use futures_util::Future;

use crate::{async_world, node_builder::TaskHolder, spawn, NodeBuilder, UiRoot};

pub struct RawHaalkaEl {
    pub(crate) node_builder: Option<NodeBuilder>,
    pub(crate) deferred_updaters: Vec<Box<dyn FnOnce(RawHaalkaEl) -> RawHaalkaEl + Send + 'static>>,
}

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            node_builder: Some(NodeBuilder::from(node_bundle)),
            ..Self::new_dummy()
        }
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

    pub fn new() -> Self {
        Self {
            node_builder: Some(default()),
            deferred_updaters: default(),
        }
    }

    // force updates to be in the back of the line, with some crude ordering
    // TODO: allow attaching some sortable data to unlimit ordering options
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

    pub fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder) -> NodeBuilder) -> Self {
        self.node_builder = Some(updater(self.node_builder.unwrap()));
        self
    }

    pub fn into_node_builder(mut self) -> NodeBuilder {
        let deferred_updaters = self.deferred_updaters.drain(..).collect::<Vec<_>>();
        let mut self_ = self;
        for updater in deferred_updaters {
            self_ = self_.update_raw_el(updater);
        }
        self_.node_builder.unwrap()
    }

    pub fn insert<B: Bundle>(self, bundle: B) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.insert(bundle)))
    }

    pub fn child<IORE: IntoOptionRawElement>(self, child_option: IORE) -> Self {
        if let Some(child) = child_option.into_option_element() {
            return self.update_node_builder(|node_builder| node_builder.child(child.into_raw().into_node_builder()));
        }
        self
    }

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

    pub fn on_spawn(self, on_spawn: impl FnOnce(&mut World, Entity) + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.on_spawn(on_spawn)))
    }

    /// TODO: requires bevy 0.14
    // pub fn on_remove(self, on_remove: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) ->
    // Self { self.on_spawn(|world, entity| {
    //         if let Some(mut on_remove_component) = world.entity_mut(entity).get_mut::<OnRemove>() {
    //             on_remove_component.0.push(Box::new(on_remove));
    //         } else {
    //             world.entity_mut(entity).insert(OnRemove(vec![Box::new(on_remove)]));
    //         }
    //     })
    // }

    pub fn on_event_with_system<E: EntityEvent, Marker>(self, handler: impl IntoSystem<(), (), Marker>) -> Self {
        self.insert(On::<E>::run(handler))
    }

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

    pub fn on_event<E: EntityEvent>(self, mut handler: impl FnMut(Listener<E>) + Send + Sync + 'static) -> Self {
        self.on_event_with_system::<E, _>(move |event: Listener<E>| handler(event))
    }

    pub fn on_event_mut<E: EntityEvent>(self, mut handler: impl FnMut(ListenerMut<E>) + Send + Sync + 'static) -> Self {
        self.on_event_with_system::<E, _>(move |event: ListenerMut<E>| handler(event))
    }

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

    pub fn on_event_stop_propagation<E: EntityEvent>(self, handler: impl FnMut(&E) + Send + Sync + 'static) -> Self {
        self.on_event_propagation_stoppable::<E>(handler, always(true))
    }

    // global in relation to the ui root
    pub fn on_global_event_with_system<E: EntityEvent, Marker>(
        self,
        handler: impl IntoSystem<(), (), Marker> + Send + 'static,
    ) -> Self {
        self.insert_forwarded(ui_root_forwarder, On::<E>::run(handler))
    }

    pub fn on_global_event<E: EntityEvent>(self, mut handler: impl FnMut(Listener<E>) + Send + Sync + 'static) -> Self {
        self.on_global_event_with_system::<E, _>(move |event: Listener<E>| handler(event))
    }

    pub fn on_global_event_mut<E: EntityEvent>(
        self,
        mut handler: impl FnMut(ListenerMut<E>) + Send + Sync + 'static,
    ) -> Self {
        self.on_global_event_with_system::<E, _>(move |event: ListenerMut<E>| handler(event))
    }

    pub fn on_signal<T, Fut: Future<Output = ()> + Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        f: impl FnMut(Entity, T) -> Fut + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.on_signal(signal, f)))
    }

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

    pub fn with_entity(self, f: impl FnOnce(EntityWorldMut) + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.with_entity(f)))
    }

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

    pub fn insert_forwarded(
        self,
        forwarder: impl FnOnce(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        bundle: impl Bundle + Send + 'static,
    ) -> Self {
        self.with_entity_forwarded(forwarder, move |mut entity| {
            entity.insert(bundle);
        })
    }

    pub fn with_component<C: Component>(self, f: impl FnOnce(&mut C) + Send + 'static) -> Self {
        self.with_entity(|mut entity| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component);
            }
        })
    }

    pub fn with_component_forwarded<C: Component>(
        self,
        forwarder: impl FnOnce(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        f: impl FnOnce(&mut C) + Send + 'static,
    ) -> Self {
        self.with_entity_forwarded(forwarder, move |mut entity| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component);
            }
        })
    }

    pub fn hold_tasks(self, tasks: impl IntoIterator<Item = Task<()>> + Send + 'static) -> Self {
        self.with_component::<TaskHolder>(|task_holder| {
            for task in tasks.into_iter() {
                task_holder.hold(task);
            }
        })
    }

    pub fn on_signal_with_entity<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        f: impl FnMut(EntityWorldMut, T) + Send + 'static,
    ) -> Self {
        let f = Arc::new(Mutex::new(f));
        self.on_signal(signal, move |entity, value| {
            async_world().apply(clone!((f) move |world: &mut World| {
                if let Some(entity) = world.get_entity_mut(entity) {
                    // safe because commands are run serially  // TODO: confirm, otherwise f must be Clone
                    (f.lock().expect("expected on_signal commands to run serially"))(entity, value);
                }
            }))
        })
    }

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

    pub fn on_signal_with_component<T: Send + 'static, C: Component>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(&mut C, T) + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity(signal, move |mut entity, value| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component, value);
            }
        })
    }

    pub fn on_signal_with_component_forwarded<T: Send + 'static, C: Component>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        forwarder: impl FnMut(&mut EntityWorldMut) -> Option<Entity> + Send + 'static,
        mut f: impl FnMut(&mut C, T) + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity_forwarded(signal, forwarder, move |mut entity, value| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component, value);
            }
        })
    }

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

    pub fn on_signal_send_event<T, E: Event>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut event_f: impl FnMut(Entity, T) -> E + Send + 'static,
    ) -> Self {
        self.on_signal(signal, move |entity, value| {
            async_world().send_event(event_f(entity, value))
        })
    }

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

    pub fn on_signal_one_shot_io_with_component<I: Send + 'static, O: Send + 'static, M, C: Component>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), O, M> + Send + 'static,
        mut f: impl FnMut(&mut C, O) + Send + 'static,
    ) -> Self {
        self.on_signal_one_shot_io_with_entity(signal, system, move |mut entity, value| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component, value);
            }
        })
    }

    pub fn on_signal_one_shot<I: Send + 'static, M>(
        self,
        signal: impl Signal<Item = I> + Send + 'static,
        system: impl IntoSystem<(Entity, I), (), M> + Send + 'static,
    ) -> Self {
        self.on_signal_one_shot_io(signal, system, |_, _| async {})
    }

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
}

fn ui_root_forwarder(entity: &mut EntityWorldMut) -> Option<Entity> {
    entity.world_scope(|world| world.get_resource::<UiRoot>().map(|&UiRoot(ui_root)| ui_root))
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

pub trait RawElement: Sized {
    fn into_raw(self) -> RawHaalkaEl;
}

impl<REW: RawElWrapper> RawElement for REW {
    fn into_raw(self) -> RawHaalkaEl {
        self.into_raw_el().into()
    }
}

pub trait IntoRawElement {
    type EL: RawElement;
    fn into_raw_element(self) -> Self::EL;
}

impl<T: RawElement> IntoRawElement for T {
    type EL = T;
    fn into_raw_element(self) -> Self::EL {
        self
    }
}

pub trait IntoOptionRawElement {
    type EL: RawElement;
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

pub trait RawElWrapper: Sized {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl;

    fn update_raw_el(mut self, updater: impl FnOnce(RawHaalkaEl) -> RawHaalkaEl) -> Self {
        let raw_el = mem::replace(self.raw_el_mut(), RawHaalkaEl::new_dummy());
        mem::swap(self.raw_el_mut(), &mut updater(raw_el));
        self
    }

    fn into_raw_el(mut self) -> RawHaalkaEl {
        mem::replace(self.raw_el_mut(), RawHaalkaEl::new_dummy())
    }
}

pub trait Spawnable: RawElWrapper {
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw_el().into_node_builder().spawn(world)
    }
}

impl<REW: RawElWrapper> Spawnable for REW {}
