use std::{
    mem,
    sync::{Arc, Mutex},
};

use bevy::{prelude::*, tasks::Task};
use enclose::enclose as clone;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};
use futures_util::Future;

use crate::{
    align::{AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable, ChildProcessable},
    async_world,
    node_builder::TaskHolder,
    pointer_event_aware::MouseInteractionAware,
    NodeBuilder,
};

// TODO: how can i make use of this default ? should i just remove it ?
pub struct RawHaalkaEl<NodeType = NodeBundle> {
    node_builder: Option<NodeBuilder<NodeType>>,
}

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            node_builder: Some(NodeBuilder::from(node_bundle)),
            ..Self::new_dummy()
        }
    }
}

impl<NodeType: Bundle + Default> RawHaalkaEl<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawHaalkaEl<NodeType> {
    fn new_dummy() -> Self {
        Self { node_builder: None }
    }

    pub fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder<NodeType>) -> NodeBuilder<NodeType>) -> Self {
        self.node_builder = Some(updater(self.node_builder.unwrap()));
        self
    }

    pub fn into_node_builder(self) -> NodeBuilder<NodeType> {
        self.node_builder.unwrap()
    }

    pub fn child<IORE: IntoOptionRawElement>(self, child_option: IORE) -> Self
    where
        <IORE::EL as RawElement>::NodeType: Bundle,
    {
        if let Some(child) = child_option.into_option_element() {
            return self.update_node_builder(|node_builder| node_builder.child(child.into_raw().into_node_builder()));
        }
        self
    }

    pub fn child_signal<IORE: IntoOptionRawElement>(
        self,
        child_option_signal: impl Signal<Item = IORE> + Send + 'static,
    ) -> Self
    where
        <IORE::EL as RawElement>::NodeType: Bundle,
    {
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
        <IORE::EL as RawElement>::NodeType: Bundle,
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
    ) -> Self
    where
        <IORE::EL as RawElement>::NodeType: Bundle,
    {
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
        self.on_spawn(move |world, entity| {
            if let Some(mut entity) = world.get_entity_mut(entity) {
                f(&mut entity);
            }
        })
    }

    pub fn with_component<C: Component>(self, f: impl FnOnce(&mut C) + Send + 'static) -> Self {
        self.with_entity(|entity| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component);
            }
        })
    }

    pub fn insert<B: Bundle>(self, bundle: B) -> Self {
        self.with_entity(|entity| {
            entity.insert(bundle);
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
        signal: impl Signal<Item = T> + 'static + Send,
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

    pub fn on_signal_with_component<C: Component, T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + 'static + Send,
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
        component_signal: impl Signal<Item = impl Into<Option<C>>> + 'static + Send,
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
}

pub trait RawElement: Sized {
    type NodeType: Bundle;
    fn into_raw(self) -> RawHaalkaEl<Self::NodeType>;
}

impl<REW: RawElWrapper> RawElement for REW {
    type NodeType = REW::NodeType;
    fn into_raw(self) -> RawHaalkaEl<Self::NodeType> {
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
    type NodeType: Bundle;

    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<Self::NodeType>;

    fn update_raw_el(
        mut self,
        updater: impl FnOnce(RawHaalkaEl<Self::NodeType>) -> RawHaalkaEl<Self::NodeType>,
    ) -> Self {
        let raw_el = mem::replace(self.raw_el_mut(), RawHaalkaEl::<Self::NodeType>::new_dummy());
        mem::swap(self.raw_el_mut(), &mut updater(raw_el));
        self
    }

    fn into_raw_el(mut self) -> RawHaalkaEl<Self::NodeType> {
        mem::replace(self.raw_el_mut(), RawHaalkaEl::<Self::NodeType>::new_dummy())
    }
}

pub trait ElementWrapper {
    type EL: RawElWrapper + ChildAlignable;
    fn element_mut(&mut self) -> &mut Self::EL;
}

impl<EW: ElementWrapper> RawElWrapper for EW {
    type NodeType = <<EW as ElementWrapper>::EL as RawElWrapper>::NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<Self::NodeType> {
        self.element_mut().raw_el_mut()
    }
}

pub trait IntoElement {
    type EL: Element;
    fn into_element(self) -> Self::EL;
}

impl<T: Element> IntoElement for T {
    type EL = T;
    fn into_element(self) -> Self::EL {
        self
    }
}

pub trait IntoOptionElement {
    type EL: Element;
    fn into_option_element(self) -> Option<Self::EL>;
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for Option<IE> {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        self.map(|into_element| into_element.into_element())
    }
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for IE {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        Some(self.into_element())
    }
}

impl<EW: ElementWrapper> Alignable for EW {
    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        self.element_mut().align_mut()
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        EW::EL::apply_content_alignment(style, alignment, action);
    }
}

impl<EW: ElementWrapper + Alignable + 'static> ChildAlignable for EW {
    fn update_style(style: &mut Style) {
        EW::EL::update_style(style);
    }

    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove) {
        EW::EL::apply_alignment(style, align, action);
    }
}

impl<NodeType: Bundle> RawElWrapper for RawHaalkaEl<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self
    }
}

pub trait Spawnable: RawElWrapper {
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw_el().into_node_builder().spawn(world)
    }
}

impl<REW: RawElWrapper> Spawnable for REW {}

impl<REW: RawElWrapper> MouseInteractionAware for REW {}

pub trait Element: RawElement + ChildProcessable {}

impl<T: RawElement + ChildProcessable> Element for T {}
