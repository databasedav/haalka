use std::{future::Future, mem};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task}, ui::{FocusPolicy, widget::{TextFlags, UiImageSize}, ContentSize}, text::TextLayoutInfo,
};
use futures_signals::{signal::{Mutable, Signal, SignalExt}, signal_vec::{SignalVec, SignalVecExt, VecDiff, MutableVec}};
use bevy_async_ecs::*;
pub use enclose::enclose as clone;
use futures_signals_ext::MutableExt;


// static ASYNC_WORLD: OnceLock<AsyncWorld> = OnceLock::new();

// fn async_world() -> &'static AsyncWorld {
//     ASYNC_WORLD.get().expect("expected AsyncWorld to be initialized")
// }

#[derive(Default)]
pub struct NodeBuilder<NodeType> {
    pub raw_node: NodeType,
    on_spawns: Vec<Box<dyn FnOnce(&mut World, Entity) + Send + Sync>>,
    task_wrappers: Vec<Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send + Sync>>,
    contiguous_child_block_populations: MutableVec<usize>,
    child_block_inserted: MutableVec<bool>,
}

impl<T: Bundle> From<T> for NodeBuilder<T> {
    fn from(node_bundle: T) -> Self {
        NodeBuilder {
            raw_node: node_bundle,
            on_spawns: default(),
            task_wrappers: default(),
            contiguous_child_block_populations: default(),
            child_block_inserted: default(),
        }
    }
}

macro_rules! impl_node_methods {
    ($($node_type:ty => [$($field:ident: $field_type:ty),* $(,)?]),+ $(,)?) => {
        $(
            impl El<$node_type> {
                $(
                    pub fn $field(self, $field: impl Signal<Item = $field_type> + Send + Sync + 'static) -> Self {
                        self.component_signal($field)
                    }
                )*
            }
        )*
    };
}

impl_node_methods! {
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

impl<NodeType: Bundle> NodeBuilder<NodeType> {
    pub fn on_spawn(mut self, on_spawn: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) -> Self {
        self.on_spawns.push(Box::new(on_spawn));
        self
    }

    pub fn insert<T: Bundle>(self, bundle: T) -> Self {
        self.on_spawn(|world: &mut World, entity: Entity| {
            if let Some(mut entity) = world.get_entity_mut(entity) {
                entity.insert(bundle);
            }
        })
    }

    pub fn component_signal(mut self, component_signal: impl Signal<Item = impl Component> + Send + Sync + 'static) -> Self {
        self.task_wrappers.push(Box::new(sync_component_task_wrapper(component_signal)));
        self
    }

    // TODO: list out limitations; limitation: if multiple children are added to entity, they must be registered thru this abstraction because of the way siblings are tracked
    pub fn child<ChildNodeType: Bundle>(mut self, child: NodeBuilder<ChildNodeType>) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(0);
        self.child_block_inserted.lock_mut().push(false);
        let child_block_inserted = self.child_block_inserted.clone();
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &child_block_inserted).await;
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
                    contiguous_child_block_populations.lock_mut().set(block, 1);
                    child_block_inserted.lock_mut().set(block, true);
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn child_signal<ChildNodeType: Bundle>(mut self, child_option: impl Signal<Item = impl Into<Option<NodeBuilder<ChildNodeType>>> + Send> + Send + Sync + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(0);
        self.child_block_inserted.lock_mut().push(false);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let child_block_inserted = self.child_block_inserted.clone();
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            let offset = offset(block, &contiguous_child_block_populations);
            let existing_child_option = Mutable::new(None);
            spawn(clone!((async_world, entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &child_block_inserted).await;
                }
                child_option.for_each(move |child_option| {
                    clone!((async_world, existing_child_option, offset, child_block_inserted, contiguous_child_block_populations) async move {
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
                                contiguous_child_block_populations.lock_mut().set(block, 1);
                                child_block_inserted.lock_mut().set(block, true);
                            }).await;
                        } else {
                            async_world.apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();
                                    }
                                }
                                contiguous_child_block_populations.lock_mut().set(block, 0);
                                child_block_inserted.lock_mut().set(block, true);
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

    pub fn children<ChildNodeType: Bundle>(mut self, children: impl IntoIterator<Item = NodeBuilder<ChildNodeType>> + Send + Sync + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(0);
        self.child_block_inserted.lock_mut().push(false);
        let child_block_inserted = self.child_block_inserted.clone();
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &child_block_inserted).await;
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
                    contiguous_child_block_populations.lock_mut().set(block, population);
                    child_block_inserted.lock_mut().set(block, true);
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn children_signal_vec<ChildNodeType: Bundle>(mut self, children_signal_vec: impl SignalVec<Item = NodeBuilder<ChildNodeType>> + Send + Sync + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(0);
        self.child_block_inserted.lock_mut().push(false);
        let child_block_inserted = self.child_block_inserted.clone();
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) {
                let children_entities = MutableVec::default();
                children_signal_vec
                .for_each(clone!((async_world, parent, children_entities, offset, contiguous_child_block_populations, child_block_inserted) move |diff| {
                    clone!((async_world, parent, children_entities, offset, contiguous_child_block_populations, child_block_inserted) async move {
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
                                        contiguous_child_block_populations.lock_mut().set(block, children_lock.len());
                                    } else {  // parent despawned during child spawning
                                        for entity in children_lock.drain(..) {
                                            if let Some(child) = world.get_entity_mut(entity) {
                                                child.despawn_recursive();
                                            }
                                        }
                                    }
                                    child_block_inserted.lock_mut().set(block, true);
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
                                        contiguous_child_block_populations.lock_mut().set(block, children_lock.len());
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                    child_block_inserted.lock_mut().set(block, true);
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
                                        contiguous_child_block_populations.lock_mut().set(block, children_lock.len());
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                    child_block_inserted.lock_mut().set(block, true);
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
                                    child_block_inserted.lock_mut().set(block, true);
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
                                    child_block_inserted.lock_mut().set(block, true);
                                })
                                .await;
                            }
                            VecDiff::RemoveAt { index } => {
                                async_world.apply(move |world: &mut World| {
                                    if let Some(existing_child) = children_entities.lock_ref().get(index).copied() {
                                        if let Some(child) = world.get_entity_mut(existing_child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                        let mut children_lock = children_entities.lock_mut();
                                        children_lock.remove(index);
                                        contiguous_child_block_populations.lock_mut().set(block, children_lock.len());
                                    }
                                    child_block_inserted.lock_mut().set(block, true);
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
                                        contiguous_child_block_populations.lock_mut().set(block, children_lock.len());
                                    }
                                    child_block_inserted.lock_mut().set(block, true);
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
                                    contiguous_child_block_populations.lock_mut().set(block, children_lock.len());
                                    child_block_inserted.lock_mut().set(block, true);
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
        let id = world.spawn(self.raw_node).id();
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
                entity.insert(TaskHolder(tasks));
            }
        }
        id
    }
}

// TODO: how do i use this default
pub struct RawHaalkaEl<NodeType = NodeBundle>(pub Option<NodeBuilder<NodeType>>);

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(Some(NodeBuilder::from(node_bundle)))
    }
}

impl<NodeType: Bundle + Default> RawHaalkaEl<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawHaalkaEl<NodeType> {
    fn new_dummy() -> Self {
        Self(None)
    }

    fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder<NodeType>) -> NodeBuilder<NodeType>) -> Self {
        self.0 = Some(updater(self.0.unwrap()));
        self
    }

    fn on_spawn(self, on_spawn: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) -> Self {
        self.update_node_builder(|node_builder| node_builder.on_spawn(on_spawn))
    }

    fn insert<T: Bundle>(self, bundle: T) -> Self {
        self.update_node_builder(|node_builder| node_builder.insert(bundle))
    }

    fn child<IOE: IntoOptionElement>(self, child_option: IOE) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        if let Some(child) = child_option.into_option_element() {
            return self.update_node_builder(|node_builder| node_builder.child(child.into_raw().into_node_builder()))
        }
        self
    }

    fn child_signal<IOE: IntoOptionElement>(self, child_option_signal: impl Signal<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.update_node_builder(|node_builder| {
            node_builder
            .child_signal(child_option_signal.map(|child_option| {
                child_option.into_option_element()
                .map(|child| child.into_raw().into_node_builder())
            })
        )})
    }

    fn children<IOE: IntoOptionElement, I: IntoIterator<Item = IOE>>(self, children_options: I) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle, I::IntoIter: Send + Sync + 'static
    {
        self.update_node_builder(|node_builder| {
            node_builder.children(
                children_options.into_iter()
                .filter_map(|child_option| child_option.into_option_element())
                .map(|child| child.into_element().into_raw().into_node_builder())
            )
        })
    }

    fn children_signal_vec<IOE: IntoOptionElement>(self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.update_node_builder(|node_builder| {
            node_builder.children_signal_vec(
                children_options_signal_vec
                .filter_map(|child_option| child_option.into_option_element())
                .map(|child| child.into_element().into_raw().into_node_builder())
            )
        })
    }

    fn component_signal(self, component_signal: impl Signal<Item = impl Component> + 'static + Send + Sync) -> Self {
        self.update_node_builder(|node_builder| node_builder.component_signal(component_signal))
    }

    fn spawn(self, world: &mut World) -> Entity {
        self.into_node_builder().spawn(world)
    }

    fn into_node_builder(self) -> NodeBuilder<NodeType> {
        self.0.unwrap()
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
    type NodeType;
    type EL: Element;
    fn into_option_element(self) -> Option<Self::EL>;
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for Option<IE> {
    type NodeType = E::NodeType;
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        self.map(|into_element| into_element.into_element())
    }
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for IE {
    type NodeType = E::NodeType;
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

impl<NodeType: Bundle> Element for RawHaalkaEl<NodeType> {
    type NodeType = NodeType;
    fn into_raw(self) -> Self {
        self
    }
}

pub struct El<NodeType>(RawHaalkaEl<NodeType>);

// TODO: r functions like this possible?
// fn get_component_mut<'a, C: Component>(world: &'a mut World, entity: Entity) -> Option<&mut Mut<'_, C>> {
//     if let Some(entity) = world.get_entity_mut(entity).as_mut() {
//         return entity.get_mut::<C>().as_mut();
//     }
//     None
// }

impl<NodeType: Bundle> From<NodeType> for El<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .on_spawn(|world, entity| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    if let Some(mut style) = entity.get_mut::<Style>() {
                        style.display = Display::Flex;
                    }
                }
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
        &mut self.0
    }
}

impl<NodeType: Bundle> El<NodeType> {
    pub fn child<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn child_signal<IOE: IntoOptionElement>(mut self, child_option: impl Signal<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn children<IOE: IntoOptionElement, I: IntoIterator<Item = IOE>>(mut self, children: I) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle, I::IntoIter: Send + Sync + 'static
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn children_signal_vec<IOE: IntoOptionElement>(mut self, children_signal_vec: impl SignalVec<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

pub struct Column<NodeType>(RawHaalkaEl<NodeType>);  // TODO: impl Element like api so the inner raw el's don't need to be managed

impl<NodeType: Bundle> From<NodeType> for Column<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .on_spawn(|world, entity| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    if let Some(mut style) = entity.get_mut::<Style>() {
                        style.display = Display::Flex;
                        style.flex_direction = FlexDirection::Column;
                    }
                }
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
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn item_signal<IOE: IntoOptionElement>(mut self, child_option: impl Signal<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn items<IOE: IntoOptionElement, I: IntoIterator<Item = IOE>>(mut self, children: I) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle, I::IntoIter: Send + Sync + 'static
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn items_signal_vec<IOE: IntoOptionElement>(mut self, children_signal_vec: impl SignalVec<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Column<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        &mut self.0
    }
}

pub struct Row<NodeType>(RawHaalkaEl<NodeType>);

impl<NodeType: Bundle> From<NodeType> for Row<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .on_spawn(|world, entity| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    if let Some(mut style) = entity.get_mut::<Style>() {
                        style.display = Display::Flex;
                        style.flex_direction = FlexDirection::Row;
                    }
                }
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
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn item_signal<IOE: IntoOptionElement>(mut self, child_option: impl Signal<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn items<IOE: IntoOptionElement, I: IntoIterator<Item = IOE>>(mut self, children: I) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle, I::IntoIter: Send + Sync + 'static
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn items_signal_vec<IOE: IntoOptionElement>(mut self, children_signal_vec: impl SignalVec<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Row<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        &mut self.0
    }
}

pub struct Stack<NodeType>(RawHaalkaEl<NodeType>);

impl<NodeType: Bundle> From<NodeType> for Stack<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(
            RawHaalkaEl::from(node_bundle)
            .on_spawn(|world, entity| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    if let Some(mut style) = entity.get_mut::<Style>() {
                        style.display = Display::Grid;
                        style.grid_auto_columns = vec![GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto)];
                        style.grid_auto_rows = vec![GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto)];
                    }
                }
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
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn layer_signal<IOE: IntoOptionElement>(mut self, child_option: impl Signal<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn layers<IOE: IntoOptionElement, I: IntoIterator<Item = IOE>>(mut self, children: I) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle, I::IntoIter: Send + Sync + 'static
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn layers_signal_vec<IOE: IntoOptionElement>(mut self, children_signal_vec: impl SignalVec<Item = IOE> + Send + Sync + 'static) -> Self
    where <<IOE as IntoOptionElement>::EL as Element>::NodeType: Bundle
    {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Stack<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        &mut self.0
    }
}

pub trait OnSpawnable: RawElWrapper + Sized {
    fn on_spawn(self, on_spawn: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.on_spawn(on_spawn))
    }
}

impl<NodeType: Bundle> OnSpawnable for El<NodeType> {}
impl<NodeType: Bundle> OnSpawnable for Column<NodeType> {}
impl<NodeType: Bundle> OnSpawnable for Row<NodeType> {}
impl<NodeType: Bundle> OnSpawnable for Stack<NodeType> {}

pub trait Insertable: RawElWrapper + Sized {
    fn insert<T: Bundle>(self, bundle: T) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert(bundle))
    }
}

impl<NodeType: Bundle> Insertable for El<NodeType> {}
impl<NodeType: Bundle> Insertable for Column<NodeType> {}
impl<NodeType: Bundle> Insertable for Row<NodeType> {}
impl<NodeType: Bundle> Insertable for Stack<NodeType> {}

pub trait MouseInteractionAware: RawElWrapper + Sized {
    fn on_hovered_change(self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.insert(Hoverable(Box::new(handler)))))
    }

    fn on_press(self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.insert(Pressable(Box::new(handler)))))
    }
}

impl MouseInteractionAware for El<ButtonBundle> {}
impl MouseInteractionAware for Column<ButtonBundle> {}
impl MouseInteractionAware for Row<ButtonBundle> {}
impl MouseInteractionAware for Stack<ButtonBundle> {}

pub trait Spawnable: RawElWrapper + Sized {
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw_el().spawn(world)
    }
}

impl<NodeType: Bundle> Spawnable for El<NodeType> {}
impl<NodeType: Bundle> Spawnable for Column<NodeType> {}
impl<NodeType: Bundle> Spawnable for Row<NodeType> {}
impl<NodeType: Bundle> Spawnable for Stack<NodeType> {}

pub trait ComponentSignalable: RawElWrapper + Sized {
    fn component_signal(self, component_signal: impl Signal<Item = impl Component> + 'static + Send + Sync) -> Self {
        self.update_raw_el(|raw_el| raw_el.component_signal(component_signal))
    }
}

impl<NodeType: Bundle> ComponentSignalable for El<NodeType> {}
impl<NodeType: Bundle> ComponentSignalable for Column<NodeType> {}
impl<NodeType: Bundle> ComponentSignalable for Row<NodeType> {}
impl<NodeType: Bundle> ComponentSignalable for Stack<NodeType> {}

#[derive(Component)]
struct Hoverable(Box<dyn FnMut(bool) + 'static + Send + Sync>);

#[derive(Component)]
struct Pressable(Box<dyn FnMut(bool) + 'static + Send + Sync>);

#[derive(Component)]
struct TaskHolder(Vec<Task<()>>);

fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}

async fn sync_component<T: Component>(async_world: AsyncWorld, entity: Entity, component_signal: impl Signal<Item = T> + 'static + Send + Sync) {
    // TODO: need partial_eq derivations for all the node related components to minimize updates with .dedupe
    component_signal.for_each(|value| {
        clone!((async_world) async move {
            async_world.apply(move |world: &mut World| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    entity.insert(value);
                }
            }).await;
        })
    }).await;
}

fn sync_component_task_wrapper<T: Component>(component_signal: impl Signal<Item = T> + 'static + Send + Sync) -> Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send + Sync> {
    Box::new(|async_world: AsyncWorld, entity: Entity| {
        spawn(sync_component(async_world, entity, component_signal))
    })
}

fn get_offset(i: usize, contiguous_child_block_populations: &[usize]) -> usize {
    contiguous_child_block_populations[0..i].iter().sum()
}

fn offset(i: usize, contiguous_child_block_populations: &MutableVec<usize>) -> Mutable<usize> {
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

async fn wait_until_child_block_inserted(block: usize, child_block_inserted: &MutableVec<bool>) {
    child_block_inserted.signal_vec().to_signal_map(|last_child_block_inserted| last_child_block_inserted[block]).wait_for(true).await;
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
