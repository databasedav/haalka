use std::{
    mem,
    sync::{Arc, Mutex},
};

use async_lock;
use bevy::{prelude::*, tasks::Task};
use enclose::enclose as clone;
use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};
use futures_signals_ext::*;
use futures_util::Future;

use crate::{async_world, node_builder::TaskHolder, spawn, NodeBuilder};

pub struct RawHaalkaEl {
    pub(crate) node_builder: Option<NodeBuilder>,
}

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            node_builder: Some(NodeBuilder::from(node_bundle)),
            ..Self::new_dummy()
        }
    }
}

impl RawHaalkaEl {
    fn new_dummy() -> Self {
        Self { node_builder: None }
    }

    pub fn new() -> Self {
        Self {
            node_builder: Some(default()),
        }
    }

    pub fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder) -> NodeBuilder) -> Self {
        self.node_builder = Some(updater(self.node_builder.unwrap()));
        self
    }

    pub fn into_node_builder(self) -> NodeBuilder {
        self.node_builder.unwrap()
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

    pub fn with_entity(self, f: impl FnOnce(&mut EntityWorldMut) + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.with_entity(f)))
    }

    pub fn with_component<C: Component>(self, f: impl FnOnce(&mut C) + Send + 'static) -> Self {
        self.with_entity(|entity| {
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
        f: impl FnMut(&mut EntityWorldMut, T) + Send + 'static,
    ) -> Self {
        let f = Arc::new(Mutex::new(f));
        self.on_signal(signal, move |entity, value| {
            async_world().apply(clone!((f) move |world: &mut World| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    // safe because commands are run serially  // TODO: confirm, otherwise f must be Clone
                    (f.lock().expect("expected on_signal commands to run serially"))(&mut entity, value);
                }
            }))
        })
    }

    pub fn on_signal_with_component<T: Send + 'static, C: Component>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(&mut C, T) + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity(signal, move |entity, value| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component, value);
            }
        })
    }

    pub fn component_signal<C: Component>(
        self,
        component_signal: impl Signal<Item = impl Into<Option<C>>> + Send + 'static,
    ) -> Self {
        // TODO: need partial_eq derivations for all the node related components to minimize updates
        // with .dedupe
        self.on_signal_with_entity::<Option<C>>(
            component_signal.map(|into_component_option| into_component_option.into()),
            move |entity, component_option| {
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
        .on_signal(signal, move |entity, input| {
            clone!((system_holder, f) async move {
                system_holder.signal_ref(Option::is_some).wait_for(true).await;
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
        f: impl FnMut(&mut EntityWorldMut, O) + Send + 'static,
    ) -> Self {
        let f = Arc::new(Mutex::new(f));
        self.on_signal_one_shot_io(signal, system, move |entity, value| {
            async_world().apply(clone!((f) move |world: &mut World| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    f.lock().unwrap()(&mut entity, value);
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
        self.on_signal_one_shot_io_with_entity(signal, system, move |entity, value| {
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
        self.on_signal_one_shot_io_with_entity(signal, system, |entity, into_option_component| {
            if let Some(component) = into_option_component.into() {
                entity.insert(component);
            } else {
                entity.remove::<C>();
            }
        })
    }
}

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
