use std::{future::Future, mem, collections::BTreeSet, convert::identity};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task}, ui::{FocusPolicy, widget::{TextFlags, UiImageSize}, ContentSize}, text::TextLayoutInfo,
};
pub use futures_signals::{self, signal::{Mutable, Signal, SignalExt}, signal_vec::{SignalVec, SignalVecExt, VecDiff, MutableVec}};
use bevy_async_ecs::*;
pub use enclose::enclose as clone;
pub use futures_signals_ext::{self, MutableExt, BoxSignal};
use paste::paste;


// static ASYNC_WORLD: OnceLock<AsyncWorld> = OnceLock::new();

// fn async_world() -> &'static AsyncWorld {
//     ASYNC_WORLD.get().expect("expected AsyncWorld to be initialized")
// }

#[derive(Default)]
pub struct NodeBuilder<NodeType> {
    raw_node: NodeType,
    on_spawns: Vec<Box<dyn FnOnce(&mut World, Entity) + Send>>,
    task_wrappers: Vec<Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send>>,
    contiguous_child_block_populations: MutableVec<Option<usize>>,
}

impl<T: Bundle> From<T> for NodeBuilder<T> {
    fn from(node_bundle: T) -> Self {
        NodeBuilder {
            raw_node: node_bundle,
            on_spawns: default(),
            task_wrappers: default(),
            contiguous_child_block_populations: default(),
        }
    }
}

impl<NodeType: Bundle> NodeBuilder<NodeType> {
    pub fn on_spawn(mut self, on_spawn: impl FnOnce(&mut World, Entity) + Send + 'static) -> Self {
        self.on_spawns.push(Box::new(on_spawn));
        self
    }

    pub fn on_signal<T, Fut: Future<Output = ()> + Send + 'static>(mut self, signal: impl Signal<Item = T> + Send + 'static, mut f: impl FnMut(AsyncWorld, Entity, T) -> Fut + Send + 'static) -> Self {
        self.task_wrappers.push(Box::new(move |async_world: AsyncWorld, entity: Entity| {
            spawn(signal.for_each(move |value| {
                f(async_world.clone(), entity, value)
            }))
        }));
        self
    }

    // TODO: list out limitations; limitation: if multiple children are added to entity, they must be registered thru this abstraction because of the way siblings are tracked
    pub fn child<ChildNodeType: Bundle>(mut self, child: NodeBuilder<ChildNodeType>) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &contiguous_child_block_populations).await;
                }
                async_world.apply(move |world: &mut World| {
                    let child_entity = child.spawn(world);
                    if let Some(mut parent) = world.get_entity_mut(parent) {
                        parent.insert_children(offset.get(), &[child_entity]);
                    } else {  // parent despawned during child spawning
                        if let Some(child) = world.get_entity_mut(child_entity) {
                            child.despawn_recursive();
                        }
                    }
                    contiguous_child_block_populations.lock_mut().set(block, Some(1));
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn child_signal<ChildNodeType: Bundle>(mut self, child_option: impl Signal<Item = impl Into<Option<NodeBuilder<ChildNodeType>>> + Send> + Send + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            let offset = offset(block, &contiguous_child_block_populations);
            let existing_child_option = Mutable::new(None);
            spawn(clone!((async_world, entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &contiguous_child_block_populations).await;
                }
                child_option.for_each(move |child_option| {
                    clone!((async_world, existing_child_option, offset, contiguous_child_block_populations) async move {
                        if let Some(child) = child_option.into() {
                            async_world.apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();  // removes from parent
                                    }
                                }
                                let child_entity = child.spawn(world);
                                if let Some(mut parent) = world.get_entity_mut(parent) {
                                    parent.insert_children(offset.get(), &[child_entity]);
                                    existing_child_option.set(Some(child_entity));
                                } else {  // parent despawned during child spawning
                                    if let Some(child) = world.get_entity_mut(child_entity) {
                                        child.despawn_recursive();
                                    }
                                }
                                contiguous_child_block_populations.lock_mut().set(block, Some(1));
                            }).await;
                        } else {
                            async_world.apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();
                                    }
                                }
                                contiguous_child_block_populations.lock_mut().set(block, Some(0));
                            })
                            .await;
                        }
                    })
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn children<ChildNodeType: Bundle>(mut self, children: impl IntoIterator<Item = NodeBuilder<ChildNodeType>> + Send + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &contiguous_child_block_populations).await;
                }
                async_world.apply(move |world: &mut World| {
                    let mut children_entities = vec![];
                    for child in children {
                        children_entities.push(child.spawn(world));
                    }
                    let population = children_entities.len();
                    if let Some(mut parent) = world.get_entity_mut(parent) {
                        parent.insert_children(offset.get(), &children_entities);
                    } else {  // parent despawned during child spawning
                        for child in children_entities {
                            if let Some(child) = world.get_entity_mut(child) {
                                child.despawn_recursive();
                            }
                        }
                    }
                    contiguous_child_block_populations.lock_mut().set(block, Some(population));
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn children_signal_vec<ChildNodeType: Bundle>(mut self, children_signal_vec: impl SignalVec<Item = NodeBuilder<ChildNodeType>> + Send + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) {
                let children_entities = MutableVec::default();
                children_signal_vec
                .for_each(clone!((async_world, parent, children_entities, offset, contiguous_child_block_populations) move |diff| {
                    clone!((async_world, parent, children_entities, offset, contiguous_child_block_populations) async move {
                        // TODO: unit tests for every branch
                        match diff {
                            VecDiff::Replace { values: nodes } => {
                                async_world.apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    let old_children = children_lock.drain(..).collect::<Vec<_>>();
                                    for node in nodes {
                                        children_lock.push(node.spawn(world));
                                    }
                                    for child in old_children {
                                        if let Some(child) = world.get_entity_mut(child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                    }
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        parent.insert_children(offset.get(), children_lock.as_slice());
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                    } else {  // parent despawned during child spawning
                                        for entity in children_lock.drain(..) {
                                            if let Some(child) = world.get_entity_mut(entity) {
                                                child.despawn_recursive();
                                            }
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::InsertAt { index, value: node } => {
                                async_world.apply(move |world: &mut World| {
                                    let child_entity = node.spawn(world);
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        parent.insert_children(offset.get() + index, &[child_entity]);
                                        let mut children_lock = children_entities.lock_mut();
                                        children_lock.insert(index, child_entity);
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::Push { value: node } => {
                                async_world.apply(move |world: &mut World| {
                                    let child_entity = node.spawn(world);
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let mut children_lock = children_entities.lock_mut();
                                        parent.insert_children(offset.get() + children_lock.len(), &[child_entity]);
                                        children_lock.push(child_entity);
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::UpdateAt { index, value: node } => {
                                async_world.apply(move |world: &mut World| {
                                    if let Some(existing_child) = children_entities.lock_ref().get(index).copied() {
                                        if let Some(child) = world.get_entity_mut(existing_child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                    }
                                    let child_entity = node.spawn(world);
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        children_entities.lock_mut().set(index, child_entity);
                                        parent.insert_children(offset.get() + index, &[child_entity]);
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::Move { old_index, new_index } => {
                                async_world.apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    children_lock.swap(old_index, new_index);
                                    // porting the swap implementation above
                                    fn move_from_to(parent: &mut EntityWorldMut<'_>, children_entities: &[Entity], old_index: usize, new_index: usize) {
                                        if old_index != new_index {
                                            if let Some(old_entity) = children_entities.get(old_index).copied() {
                                                parent.remove_children(&[old_entity]);
                                                parent.insert_children(new_index, &[old_entity]);
                                            }
                                        }
                                    }
                                    fn swap(parent: &mut EntityWorldMut<'_>, children_entities: &[Entity], a: usize, b: usize) {
                                        move_from_to(parent, children_entities, a, b);
                                        if a < b {
                                            move_from_to(parent, children_entities, b - 1, a);

                                        } else if a > b {
                                            move_from_to(parent, children_entities, b + 1, a);
                                        }
                                    }
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let offset = offset.get();
                                        swap(&mut parent, children_lock.as_slice(), offset + old_index, offset + new_index);
                                    }
                                })
                                .await;
                            }
                            VecDiff::RemoveAt { index } => {
                                async_world.apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    if let Some(existing_child) = children_lock.get(index).copied() {
                                        if let Some(child) = world.get_entity_mut(existing_child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                        children_lock.remove(index);
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                    }
                                })
                                .await;
                            }
                            VecDiff::Pop {} => {
                                async_world.apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    if let Some(child_entity) = children_lock.pop() {
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                    }
                                })
                                .await;
                            }
                            VecDiff::Clear {} => {
                                async_world.apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    for child_entity in children_lock.drain(..) {
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                    contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                })
                                .await;
                            }
                        }
                    })
                }))
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn spawn(self, world: &mut World) -> Entity {
        // include task holder so tasks can be added on spawn
        let id = world.spawn((self.raw_node, TaskHolder::new())).id();
        for on_spawn in self.on_spawns {
            on_spawn(world, id);
        }
        if !self.task_wrappers.is_empty() {
            let mut tasks = vec![];
            let async_world = AsyncWorld::from_world(world);
            for task_wrapper in self.task_wrappers {
                tasks.push(task_wrapper(async_world.clone(), id));
            }
            if let Some(mut entity) = world.get_entity_mut(id) {
                if let Some(mut task_holder) = entity.get_mut::<TaskHolder>() {
                    for task in tasks {
                        task_holder.hold(task);
                    }
                }
            }
        }
        id
    }
}

enum AlignHolder {
    Align(Align),
    AlignSignal(BoxSignal<'static, Option<Align>>),
}

// TODO: how can i make use of this default ?
pub struct RawHaalkaEl<NodeType = NodeBundle> {
    node_builder: Option<NodeBuilder<NodeType>>,
    align: Option<AlignHolder>,
}

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self { node_builder: Some(NodeBuilder::from(node_bundle)), ..Self::new_dummy() }
    }
}

impl<NodeType: Bundle + Default> RawHaalkaEl<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawHaalkaEl<NodeType> {
    fn new_dummy() -> Self {
        Self { node_builder: None, align: None }
    }

    pub fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder<NodeType>) -> NodeBuilder<NodeType>) -> Self {
        self.node_builder = Some(updater(self.node_builder.unwrap()));
        self
    }

    pub fn into_node_builder(self) -> NodeBuilder<NodeType> {
        self.node_builder.unwrap()
    }

    pub fn child<IOE: IntoOptionElement>(self, child_option: IOE) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        if let Some(child) = child_option.into_option_element() {
            return self.update_node_builder(|node_builder| node_builder.child(child.into_raw().into_node_builder()))
        }
        self
    }

    pub fn child_signal<IOE: IntoOptionElement>(self, child_option_signal: impl Signal<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.update_node_builder(|node_builder| {
            node_builder
            .child_signal(child_option_signal.map(|child_option| {
                child_option.into_option_element()
                .map(|child| child.into_raw().into_node_builder())
            })
        )})
    }

    pub fn children<IOE: IntoOptionElement, I: IntoIterator<Item = IOE>>(self, children_options: I) -> Self
    where <IOE::EL as Element>::NodeType: Bundle, I::IntoIter: Send + 'static
    {
        self.update_node_builder(|node_builder| {
            node_builder.children(
                children_options.into_iter()
                .filter_map(|child_option| child_option.into_option_element())
                .map(|child| child.into_element().into_raw().into_node_builder())
            )
        })
    }

    pub fn children_signal_vec<IOE: IntoOptionElement>(self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.update_node_builder(|node_builder| {
            node_builder.children_signal_vec(
                children_options_signal_vec
                .filter_map(|child_option| child_option.into_option_element())
                .map(|child| child.into_element().into_raw().into_node_builder())
            )
        })
    }

    pub fn on_spawn(self, on_spawn: impl FnOnce(&mut World, Entity) + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.on_spawn(on_spawn)))
    }

    pub fn on_signal<T, Fut: Future<Output = ()> + Send + 'static>(self, signal: impl Signal<Item = T> + Send + 'static, f: impl FnMut(AsyncWorld, Entity, T) -> Fut + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.on_signal(signal, f)))
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
        f: impl FnMut(&mut EntityWorldMut, T) + Clone + Send + 'static,
    ) -> Self {
        self.on_signal(signal, move |async_world, entity, value| {
            clone!((mut f) async move {
                async_world.apply(move |world: &mut World| {
                    if let Some(mut entity) = world.get_entity_mut(entity) {
                        f(&mut entity, value);
                    }
                })
                .await;
            })
        })
    }

    pub fn on_signal_with_component<C: Component, T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + 'static + Send,
        mut f: impl FnMut(&mut C, T) + Clone + Send + 'static,
    ) -> Self {
        self.on_signal_with_entity(signal, move |entity, value| {
            if let Some(mut component) = entity.get_mut::<C>() {
                f(&mut component, value);
            }
        })
    }

    pub fn component_signal<C: Component>(self, component_signal: impl Signal<Item = C> + 'static + Send) -> Self {
        // TODO: need partial_eq derivations for all the node related components to minimize updates with .dedupe
        self.on_signal_with_entity::<C>(component_signal, move |entity, value| {
            entity.insert(value);
        })
    }
}

pub trait Element: Sized {
    type NodeType: Bundle;
    fn into_raw(self) -> RawHaalkaEl<Self::NodeType>;
}

impl<REW: RawElWrapper> Element for REW {
    type NodeType = REW::NodeType;
    fn into_raw(self) -> RawHaalkaEl<Self::NodeType> {
        self.into_raw_el().into()
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

pub trait RawElWrapper: Sized {
    type NodeType: Bundle;

    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<Self::NodeType>;

    fn update_raw_el(mut self, updater: impl FnOnce(RawHaalkaEl<Self::NodeType>) -> RawHaalkaEl<Self::NodeType>) -> Self {
        let raw_el = mem::replace(self.raw_el_mut(), RawHaalkaEl::<Self::NodeType>::new_dummy());
        mem::swap(self.raw_el_mut(), &mut updater(raw_el));
        self
    }

    fn into_raw_el(mut self) -> RawHaalkaEl<Self::NodeType> {
        mem::replace(self.raw_el_mut(), RawHaalkaEl::<Self::NodeType>::new_dummy())
    }
}

impl<NodeType: Bundle> RawElWrapper for RawHaalkaEl<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self
    }
}

pub struct El<NodeType>(RawHaalkaEl<NodeType>);

impl<NodeType: Bundle> From<NodeType> for El<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .with_component::<Style>(|style| {
                style.display = Display::Flex;
                style.flex_direction = FlexDirection::Column;
            })
        )
    }
}

impl<NodeType: Bundle + Default> El<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for El<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.0.raw_el_mut()
    }
}

impl<NodeType: Bundle> El<NodeType> {
    pub fn child<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(Self::process_child(child_option));
        self
    }

    pub fn child_signal<IOE: IntoOptionElement + 'static>(mut self, child_option: impl Signal<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option.map(Self::process_child));
        self
    }

    pub fn children<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
    where <IOE::EL as Element>::NodeType: Bundle, I::IntoIter: Send + 'static
    {
        self.0 = self.0.children(children_options.into_iter().map(Self::process_child));
        self
    }

    pub fn children_signal_vec<IOE: IntoOptionElement + 'static>(mut self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_options_signal_vec.map(Self::process_child));
        self
    }
}

pub struct Column<NodeType>(RawHaalkaEl<NodeType>);  // TODO: impl Element like api so the inner raw el's don't need to be managed

impl<NodeType: Bundle> From<NodeType> for Column<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .with_component::<Style>(|style| {
                style.display = Display::Flex;
                style.flex_direction = FlexDirection::Column;
            })
        )
    }
}

impl<NodeType: Bundle + Default> Column<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> Column<NodeType> {
    pub fn item<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(Self::process_child(child_option));
        self
    }

    pub fn item_signal<IOE: IntoOptionElement + 'static>(mut self, child_option: impl Signal<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option.map(Self::process_child));
        self
    }

    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
    where <IOE::EL as Element>::NodeType: Bundle, I::IntoIter: Send + 'static
    {
        self.0 = self.0.children(children_options.into_iter().map(Self::process_child));
        self
    }

    pub fn items_signal_vec<IOE: IntoOptionElement + 'static>(mut self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_options_signal_vec.map(Self::process_child));
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Column<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.0.raw_el_mut()
    }
}

pub struct Row<NodeType>(RawHaalkaEl<NodeType>);

impl<NodeType: Bundle> From<NodeType> for Row<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .with_component::<Style>(|style| {
                style.display = Display::Flex;
                style.flex_direction = FlexDirection::Row;
                style.align_items = AlignItems::Center;
            })
        )
    }
}

impl<NodeType: Bundle + Default> Row<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> Row<NodeType> {
    pub fn item<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(Self::process_child(child_option));
        self
    }

    pub fn item_signal<IOE: IntoOptionElement + 'static>(mut self, child_option: impl Signal<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option.map(Self::process_child));
        self
    }

    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
    where <IOE::EL as Element>::NodeType: Bundle, I::IntoIter: Send + 'static
    {
        self.0 = self.0.children(children_options.into_iter().map(Self::process_child));
        self
    }

    pub fn items_signal_vec<IOE: IntoOptionElement + 'static>(mut self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_options_signal_vec.map(Self::process_child));
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Row<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.0.raw_el_mut()
    }
}

pub struct Stack<NodeType>(RawHaalkaEl<NodeType>);

impl<NodeType: Bundle> From<NodeType> for Stack<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .with_component::<Style>(|style| {
                style.display = Display::Grid;
                style.grid_auto_columns = vec![GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto)];
                style.grid_auto_rows = vec![GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto)];
            })
        )
    }
}

impl<NodeType: Bundle + Default> Stack<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> Stack<NodeType> {
    pub fn layer<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(Self::process_child(child_option));
        self
    }

    pub fn layer_signal<IOE: IntoOptionElement + 'static>(mut self, child_option_signal: impl Signal<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option_signal.map(Self::process_child));
        self
    }

    pub fn layers<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
    where <IOE::EL as Element>::NodeType: Bundle, I::IntoIter: Send + 'static
    {
        self.0 = self.0.children(children_options.into_iter().map(Self::process_child));
        self
    }

    pub fn layers_signal_vec<IOE: IntoOptionElement + 'static>(mut self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_options_signal_vec.map(Self::process_child));
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Stack<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.0.raw_el_mut()
    }
}

pub trait MouseInteractionAware: RawElWrapper {
    fn on_hovered_change(self, handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert(Hoverable(Box::new(handler))))
    }

    fn on_pressed_change(self, handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert(Pressable(Box::new(handler))))
    }
}

impl MouseInteractionAware for RawHaalkaEl<ButtonBundle> {}
impl MouseInteractionAware for El<ButtonBundle> {}
impl MouseInteractionAware for Column<ButtonBundle> {}
impl MouseInteractionAware for Row<ButtonBundle> {}
impl MouseInteractionAware for Stack<ButtonBundle> {}

pub trait Spawnable: RawElWrapper {
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw_el().into_node_builder().spawn(world)
    }
}

impl<REW: RawElWrapper> Spawnable for REW {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alignment {
    Top,
    Bottom,
    Left,
    Right,
    CenterX,
    CenterY,
}

#[derive(Clone, Default)]
pub struct Align {
    alignments: BTreeSet<Alignment>,
}

impl Align {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn center() -> Self {
        Self::default().center_x().center_y()
    }

    pub fn center_x(mut self) -> Self {
        self.alignments.insert(Alignment::CenterX);
        self.alignments.remove(&Alignment::Left);
        self.alignments.remove(&Alignment::Right);
        self
    }

    pub fn center_y(mut self) -> Self {
        self.alignments.insert(Alignment::CenterY);
        self.alignments.remove(&Alignment::Top);
        self.alignments.remove(&Alignment::Bottom);
        self
    }

    pub fn top(mut self) -> Self {
        self.alignments.insert(Alignment::Top);
        self.alignments.remove(&Alignment::CenterY);
        self.alignments.remove(&Alignment::Bottom);
        self
    }

    pub fn bottom(mut self) -> Self {
        self.alignments.insert(Alignment::Bottom);
        self.alignments.remove(&Alignment::CenterY);
        self.alignments.remove(&Alignment::Top);
        self
    }

    pub fn left(mut self) -> Self {
        self.alignments.insert(Alignment::Left);
        self.alignments.remove(&Alignment::CenterX);
        self.alignments.remove(&Alignment::Right);
        self
    }

    pub fn right(mut self) -> Self {
        self.alignments.insert(Alignment::Right);
        self.alignments.remove(&Alignment::CenterX);
        self.alignments.remove(&Alignment::Left);
        self
    }
}

pub enum AddRemove {
    Add,
    Remove,
}

fn register_align_signal<REW: RawElWrapper>(element: REW, align_signal: impl Signal<Item = Option<Vec<Alignment>>> + Send + 'static, apply_alignment: fn(&mut Style, Alignment, AddRemove)) -> REW {
    let mut last_alignments_option: Option<Vec<Alignment>> = None;
    element.update_raw_el(|raw_el| raw_el.on_signal_with_component::<Style, Option<Vec<Alignment>>>(align_signal, move |style, aligns_option| {
        if let Some(alignments) = aligns_option {
            if let Some(mut last_alignments) = last_alignments_option.take() {
                last_alignments.retain(|align| !alignments.contains(align));
                for alignment in last_alignments {
                    apply_alignment(style, alignment, AddRemove::Remove)
                }
            }
            for alignment in &alignments {
                apply_alignment(style, *alignment, AddRemove::Add)
            }
            last_alignments_option = if !alignments.is_empty() { Some(alignments) } else { None };
        } else {
            if let Some(last_aligns) = last_alignments_option.take() {
                for align in last_aligns {
                    apply_alignment(style, align, AddRemove::Remove)
                }
            }
        }
    }))
}

pub trait ChildAlignable: RawElWrapper where Self: 'static {
    fn update_style(_style: &mut Style) {}  // only Stack requires base updates

    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove);

    fn manage<NodeType: Bundle>(mut child: RawHaalkaEl<NodeType>) -> RawHaalkaEl<NodeType> {
        child = child.with_component::<Style>(Self::update_style);
        // TODO: this .take means that child can't be passed around parents without losing align info, but this can be easily added if desired
        if let Some(align) = child.align.take() {
            match align {
                AlignHolder::Align(align) => {
                    child = child.with_component::<Style>(move |style| {
                        for align in align.alignments {
                            Self::apply_alignment(style, align, AddRemove::Add)
                        }
                    })
                }
                AlignHolder::AlignSignal(align_option_signal) => {
                    child = register_align_signal(child, align_option_signal.map(|align_option| align_option.map(|align| align.alignments.into_iter().collect())), Self::apply_alignment)
                }
            }
        }
        child
    }
}

impl<NodeType: Bundle> ChildAlignable for El<NodeType> {
    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Column::<NodeType>::apply_alignment(style, alignment, action);
    }
}

impl<NodeType: Bundle> ChildAlignable for Column<NodeType> {
    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.margin.bottom = match action {
                AddRemove::Add => Val::Auto,
                AddRemove::Remove => Val::ZERO,
            },
            Alignment::Bottom => style.margin.top = match action {
                AddRemove::Add => Val::Auto,
                AddRemove::Remove => Val::ZERO,
            },
            Alignment::Left => style.align_self = match action {
                AddRemove::Add => AlignSelf::Start,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::Right => style.align_self = match action {
                AddRemove::Add => AlignSelf::End,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::CenterX => style.align_self = match action {
                AddRemove::Add => AlignSelf::Center,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::CenterY => (style.margin.top, style.margin.bottom) = match action {
                AddRemove::Add => (Val::Auto, Val::Auto),
                AddRemove::Remove => (Val::ZERO, Val::ZERO),
            },
        }
    }
}

impl<NodeType: Bundle> ChildAlignable for Row<NodeType> {
    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.align_self = match action {
                AddRemove::Add => AlignSelf::Start,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::Bottom => style.align_self = match action {
                AddRemove::Add => AlignSelf::End,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::Left => style.margin.right = match action {
                AddRemove::Add => Val::Auto,
                AddRemove::Remove => Val::ZERO, 
            },
            Alignment::Right => style.margin.left = match action {
                AddRemove::Add => Val::Auto,
                AddRemove::Remove => Val::ZERO,
            },
            Alignment::CenterX => (style.margin.left, style.margin.right) = match action {
                AddRemove::Add => (Val::Auto, Val::Auto),
                AddRemove::Remove => (Val::ZERO, Val::ZERO),
            },
            Alignment::CenterY => style.align_self = match action {
                AddRemove::Add => AlignSelf::Center,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
        }
    }
}

impl<NodeType: Bundle> ChildAlignable for Stack<NodeType> {
    fn update_style(style: &mut Style) {
        style.grid_column = GridPlacement::start(1);
        style.grid_row = GridPlacement::start(1);
    }

    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.align_self = match action {
                AddRemove::Add => AlignSelf::Start,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::Bottom => style.align_self = match action {
                AddRemove::Add => AlignSelf::End,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::Left => style.justify_self = match action {
                AddRemove::Add => JustifySelf::Start,
                AddRemove::Remove => JustifySelf::DEFAULT,
            },
            Alignment::Right => style.justify_self = match action {
                AddRemove::Add => JustifySelf::End,
                AddRemove::Remove => JustifySelf::DEFAULT,
            },
            Alignment::CenterX => style.justify_self = match action {
                AddRemove::Add => JustifySelf::Center,
                AddRemove::Remove => JustifySelf::DEFAULT,
            },
            Alignment::CenterY => style.align_self = match action {
                AddRemove::Add => AlignSelf::Center,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
        }
    }
}

pub trait Alignable: ChildAlignable {
    fn align(self, align: Align) -> Self {
        self.update_raw_el(|mut raw_el| {
            raw_el.align = Some(AlignHolder::Align(align));
            raw_el
        })
    }

    fn align_signal(self, align_option_signal: impl Signal<Item = Option<Align>> + Send + 'static) -> Self {
        self.update_raw_el(|mut raw_el| {
            raw_el.align = Some(AlignHolder::AlignSignal(align_option_signal.boxed()));
            raw_el
        })
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove);

    fn align_content(self, align: Align) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.with_component::<Style>(|style| {
                for alignment in align.alignments {
                    Self::apply_content_alignment(style, alignment, AddRemove::Add)
                }
            })
        })
    }

    fn align_content_signal(self, align_option_signal: impl Signal<Item = Option<Align>> + Send + 'static) -> Self {
        register_align_signal(self, align_option_signal.map(|align_option| align_option.map(|align| align.alignments.into_iter().collect())), Self::apply_content_alignment)
    }
}

impl<NodeType: Bundle> Alignable for El<NodeType> {
    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Start,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Bottom => style.justify_content = match action {
                AddRemove::Add => JustifyContent::End,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Left => style.align_items = match action {
                AddRemove::Add => AlignItems::Start,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::Right => style.align_items = match action {
                AddRemove::Add => AlignItems::End,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::CenterX => style.align_items = match action {
                AddRemove::Add => AlignItems::Center,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::CenterY => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Center,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
        }
    }
}

impl<NodeType: Bundle> Alignable for Column<NodeType> {
    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Start,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Bottom => style.justify_content = match action {
                AddRemove::Add => JustifyContent::End,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Left => style.align_items = match action {
                AddRemove::Add => AlignItems::Start,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::Right => style.align_items = match action {
                AddRemove::Add => AlignItems::End,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::CenterX => style.align_items = match action {
                AddRemove::Add => AlignItems::Center,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::CenterY => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Center,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
        }
    }
}

impl<NodeType: Bundle> Alignable for Row<NodeType> {
    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.align_items = match action {
                AddRemove::Add => AlignItems::Start,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::Bottom => style.align_items = match action {
                AddRemove::Add => AlignItems::End,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::Left => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Start,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Right => style.justify_content = match action {
                AddRemove::Add => JustifyContent::End,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::CenterX => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Center,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::CenterY => style.align_items = match action {
                AddRemove::Add => AlignItems::Center,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
        }
    }
}

impl<NodeType: Bundle> Alignable for Stack<NodeType> {
    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Row::<NodeType>::apply_content_alignment(style, alignment, action)
    }
}

pub trait ChildProcessable: RawElWrapper {
    fn process_child<IOE: IntoOptionElement>(child_option: IOE) -> Option<RawHaalkaEl<<<IOE as IntoOptionElement>::EL as Element>::NodeType>>;
}

impl<CA: ChildAlignable> ChildProcessable for CA {
    fn process_child<IOE: IntoOptionElement>(child_option: IOE) -> Option<RawHaalkaEl<<<IOE as IntoOptionElement>::EL as Element>::NodeType>> {
        child_option.into_option_element().map(|child| {
            child.into_raw()
            .update_raw_el(|mut raw_el| {
                raw_el = <Self as ChildAlignable>::manage(raw_el);
                raw_el
            })
        })
    }
}

// TODO
// pub trait NearbyElementAddable: RawElWrapper {
//     fn element_below_signal;
// }

#[macro_export]
macro_rules! impl_node_methods {
    ($($el_type:ty => { $($node_type:ty => [$($field:ident: $field_type:ty),* $(,)?]),+ $(,)? }),+ $(,)?) => {
        $(
            $(
                paste! {
                    impl $el_type<$node_type> {
                        $(
                            paste! {
                                pub fn $field(self, $field: $field_type) -> Self {
                                    self.update_raw_el(|raw_el| raw_el.insert($field))
                                }

                                pub fn [<with_ $field>](self, f: impl FnOnce(&mut $field_type) + Send + 'static) -> Self {
                                    self.update_raw_el(|raw_el| raw_el.with_component::<$field_type>(f))
                                }

                                pub fn [<$field _signal>](self, [<$field _signal>]: impl Signal<Item = $field_type> + Send + 'static) -> Self {
                                    self.update_raw_el(|raw_el| raw_el.component_signal([<$field _signal>]))
                                }

                                pub fn [<on_signal_with_ $field>]<T: Send + 'static>(
                                    self,
                                    signal: impl Signal<Item = T> + Send + 'static,
                                    f: impl FnMut(&mut $field_type, T) + Clone + Send + 'static,
                                ) -> Self {
                                    self.update_raw_el(|raw_el| {
                                        raw_el.on_signal_with_component::<$field_type, T>(signal, f)
                                    })
                                }
                            }
                        )*
                    }
                }
            )*
        )*
    };
}

impl_node_methods! {
    El => {
        NodeBundle => [
            node: bevy::ui::Node,
            style: Style,
            background_color: BackgroundColor,
            border_color: BorderColor,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        ImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            image: UiImage,
            image_size: UiImageSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        AtlasImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        TextBundle => [
            node: bevy::ui::Node,
            style: Style,
            text: Text,
            text_layout_info: TextLayoutInfo,
            text_flags: TextFlags,
            calculated_size: ContentSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
            background_color: BackgroundColor,
        ],
        ButtonBundle => [
            node: bevy::ui::Node,
            button: Button,
            style: Style,
            interaction: Interaction,
            focus_policy: FocusPolicy,
            background_color: BackgroundColor,
            border_color: BorderColor,
            image: UiImage,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
    },
    Column => {
        NodeBundle => [
            node: bevy::ui::Node,
            style: Style,
            background_color: BackgroundColor,
            border_color: BorderColor,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        ImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            image: UiImage,
            image_size: UiImageSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        AtlasImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        TextBundle => [
            node: bevy::ui::Node,
            style: Style,
            text: Text,
            text_layout_info: TextLayoutInfo,
            text_flags: TextFlags,
            calculated_size: ContentSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
            background_color: BackgroundColor,
        ],
        ButtonBundle => [
            node: bevy::ui::Node,
            button: Button,
            style: Style,
            interaction: Interaction,
            focus_policy: FocusPolicy,
            background_color: BackgroundColor,
            border_color: BorderColor,
            image: UiImage,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
    },
    Row => {
        NodeBundle => [
            node: bevy::ui::Node,
            style: Style,
            background_color: BackgroundColor,
            border_color: BorderColor,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        ImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            image: UiImage,
            image_size: UiImageSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        AtlasImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        TextBundle => [
            node: bevy::ui::Node,
            style: Style,
            text: Text,
            text_layout_info: TextLayoutInfo,
            text_flags: TextFlags,
            calculated_size: ContentSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
            background_color: BackgroundColor,
        ],
        ButtonBundle => [
            node: bevy::ui::Node,
            button: Button,
            style: Style,
            interaction: Interaction,
            focus_policy: FocusPolicy,
            background_color: BackgroundColor,
            border_color: BorderColor,
            image: UiImage,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
    },
    Stack => {
        NodeBundle => [
            node: bevy::ui::Node,
            style: Style,
            background_color: BackgroundColor,
            border_color: BorderColor,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        ImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            image: UiImage,
            image_size: UiImageSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        AtlasImageBundle => [
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
        TextBundle => [
            node: bevy::ui::Node,
            style: Style,
            text: Text,
            text_layout_info: TextLayoutInfo,
            text_flags: TextFlags,
            calculated_size: ContentSize,
            focus_policy: FocusPolicy,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
            background_color: BackgroundColor,
        ],
        ButtonBundle => [
            node: bevy::ui::Node,
            button: Button,
            style: Style,
            interaction: Interaction,
            focus_policy: FocusPolicy,
            background_color: BackgroundColor,
            border_color: BorderColor,
            image: UiImage,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        ],
    },
    // TODO: macros don't play nice with generics
    // MaterialNodeBundle<M: UiMaterial> => [
    //     node: bevy::ui::Node,
    //     style: Style,
    //     focus_policy: FocusPolicy,
    //     transform: Transform,
    //     global_transform: GlobalTransform,
    //     visibility: Visibility,
    //     inherited_visibility: InheritedVisibility,
    //     view_visibility: ViewVisibility,
    //     z_index: ZIndex,
    // ],
}

#[derive(Component)]
struct Hoverable(Box<dyn FnMut(bool) + Send + Sync + 'static>);

#[derive(Component)]
struct Pressable(Box<dyn FnMut(bool) + Send + Sync + 'static>);

#[derive(Component)]
pub struct TaskHolder(Vec<Task<()>>);

impl TaskHolder {
    fn new() -> Self {
        Self(Vec::new())
    }

    pub fn hold(self: &mut Self, task: Task<()>) {
        self.0.push(task);
    }
}

pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}

fn get_offset(i: usize, contiguous_child_block_populations: &[Option<usize>]) -> usize {
    contiguous_child_block_populations[0..i].iter().copied().filter_map(identity).sum()
}

fn offset(i: usize, contiguous_child_block_populations: &MutableVec<Option<usize>>) -> Mutable<usize> {
    let offset = Mutable::new(get_offset(i, &*contiguous_child_block_populations.lock_ref()));
    let updater = {
        contiguous_child_block_populations.signal_vec()
        .to_signal_map(move |contiguous_child_block_populations| get_offset(i, contiguous_child_block_populations))
        .dedupe()
        .for_each(clone!((offset) move |new_offset| {
            offset.set_neq(new_offset);
            async {}
        }))
    };
    spawn(updater).detach();  // future dropped when all node tasks are  // TODO: confirm
    offset
}

async fn wait_until_child_block_inserted(block: usize, contiguous_child_block_populations: &MutableVec<Option<usize>>) {
    contiguous_child_block_populations.signal_vec()
    .to_signal_map(|contiguous_child_block_populations| {
        contiguous_child_block_populations.get(block).copied().flatten().is_some()
    })
    .wait_for(true).await;
}

fn hoverable_system(
    mut interaction_query: Query<(&Interaction, &mut Hoverable), Changed<Interaction>>
) {
    for (interaction, mut hoverable) in &mut interaction_query {
        hoverable.0(matches!(interaction, Interaction::Hovered));
    }
}

fn pressable_system(
    mut interaction_query: Query<(&Interaction, &mut Pressable), Changed<Interaction>>
) {
    for (interaction, mut pressable) in &mut interaction_query {
        pressable.0(matches!(interaction, Interaction::Pressed));
    }
}

pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(AsyncEcsPlugin)
        .add_systems(Update, (hoverable_system, pressable_system));
    }
}


// #[derive(Event)]
// struct MutableEvent(bool);

// fn mutable_updater_system(
//     mut interaction_query: Query<(&Interaction, &mut Pressable)>, /* Changed<Interaction>>, */  // TODO: explain the bug that occurs when using Changed
//     mut mutable_events: EventWriter<MutableEvent>,
// ) {
//     for (interaction, mut pressable) in &mut interaction_query {
//         if matches!(interaction, Interaction::Pressed) {
//             mutable_events.send(MutableEvent(true));
//             return;
//         }
//     }
//     // println!("not pressed");
//     mutable_events.send(MutableEvent(false));
// }

// fn mutable_event_listener(
//     mut mutable_events: EventReader<MutableEvent>,
//     mut mutable_holder_query: Query<&mut MutableHolder>,
// ) {
//     for mutable_event in mutable_events.read() {
//         for mut mutable_holder in &mut mutable_holder_query {
//             mutable_holder.mutable_bool.set_neq(mutable_event.0);
//         }
//     }
// }

// fn init_async_world(world: &mut World) {
//     ASYNC_WORLD.set(AsyncWorld::from_world(world)).unwrap();
//     AsyncComputeTaskPool::get_or_init(|| {
//         let task_pool = TaskPool::default();
//         task_pool.with_local_executor(|_| {
//             ASYNC_WORLD.set(AsyncWorld::from_world(world)).unwrap();
//         });
//         task_pool
//     });
// }
