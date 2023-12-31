use std::{future::Future, marker::PhantomData, mem};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task}, ui::{FocusPolicy, widget::{TextFlags, UiImageSize}, ContentSize, update}, text::TextLayoutInfo,
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
            impl El<RawHaalkaEl<$node_type>> {
                $(
                    pub fn $field(self, $field: impl Signal<Item = $field_type> + 'static + Send + Sync) -> Self {
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

impl NodeBuilder<ButtonBundle> {
    pub fn on_hovered_change(self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.insert(Hoverable(Box::new(handler)))
    }

    pub fn on_press(self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.insert(Pressable(Box::new(handler)))
    }
}

pub struct NodeTypeNotSet;

impl NodeBuilder<NodeTypeNotSet> {
    pub fn new() -> NodeBuilder<NodeBundle> {
        NodeBuilder::from(NodeBundle::default())
    }

    pub fn new_image() -> NodeBuilder<ImageBundle> {
        NodeBuilder::from(ImageBundle::default())
    }

    pub fn new_atlas_image() -> NodeBuilder<AtlasImageBundle> {
        NodeBuilder::from(AtlasImageBundle::default())
    }

    pub fn new_text() -> NodeBuilder<TextBundle> {
        NodeBuilder::from(TextBundle::default())
    }

    pub fn new_button() -> NodeBuilder<ButtonBundle> {
        NodeBuilder::from(ButtonBundle::default())
    }

    pub fn new_material<M: UiMaterial>() -> NodeBuilder<MaterialNodeBundle<M>> {
        NodeBuilder::from(MaterialNodeBundle::<M>::default())
    }
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

    pub fn component_signal(mut self, component_signal: impl Signal<Item = impl Component> + 'static + Send + Sync) -> Self {
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

    pub fn child_signal<ChildNodeType: Bundle>(mut self, child_option: impl Signal<Item = impl Into<Option<NodeBuilder<ChildNodeType>>> + Send + 'static> + 'static + Send + Sync) -> Self {
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

    pub fn children<ChildNodeType: Bundle>(mut self, children: impl IntoIterator<Item = NodeBuilder<ChildNodeType>> + 'static + Send + Sync) -> Self {
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

    pub fn children_signal_vec<ChildNodeType: Bundle>(mut self, children_signal_vec: impl SignalVec<Item = NodeBuilder<ChildNodeType>> + 'static + Send + Sync) -> Self {
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

pub trait RawEl: Sized {
    type NodeType: Bundle;

    fn new_dummy() -> Self;

    fn update_node_builder(self, updater: impl FnOnce(NodeBuilder<Self::NodeType>) -> NodeBuilder<Self::NodeType>) -> Self;

    fn child(self, child_option: impl IntoOptionElement) -> Self {
        if let Some(child) = child_option.into_option_element() {
            return self.update_node_builder(|node_builder| node_builder.child(child.into_raw().into_node_builder()))
        }
        self
    }

    fn child_signal(self, child_option_signal: impl Signal<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.update_node_builder(|node_builder| {
            node_builder
            .child_signal(child_option_signal.map(|child_option| {
                child_option.into_option_element()
                .map(|child| child.into_raw().into_node_builder())
            })
        )})
    }

    fn children<I: IntoIterator<Item = impl IntoOptionElement> + 'static + Send + Sync>(self, children_options: I) -> Self
    where I::IntoIter: Send + Sync
    {
        self.update_node_builder(|node_builder| {
            node_builder.children(
                children_options.into_iter()
                .filter_map(|child_option| child_option.into_option_element())
                .map(|child| child.into_element().into_raw().into_node_builder())
            )
        })
    }

    fn children_signal_vec(self, children_options_signal_vec: impl SignalVec<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
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

    fn into_node_builder(self) -> NodeBuilder<Self::NodeType>;
}

pub trait Element: Sized {
    type NodeType: Bundle;

    fn into_raw(self) -> RawHaalkaEl<Self::NodeType>;
}

impl<REW: RawElWrapper> Element for REW where RawHaalkaEl<<<REW as RawElWrapper>::RawEl as RawEl>::NodeType>: From<<REW as RawElWrapper>::RawEl> {
    type NodeType = <<REW as RawElWrapper>::RawEl as RawEl>::NodeType;

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

impl<E: Element, T: IntoElement<EL = E>> IntoOptionElement for Option<T> {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        self.map(|into_element| into_element.into_element())
    }
}

impl<E: Element, T: IntoElement<EL = E>> IntoOptionElement for T {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        Some(self.into_element())
    }
}

pub trait Styleable {
    fn update_style(self, updater: impl FnOnce(&mut Style)) -> Self;
}

macro_rules! impl_styleable_for_nodes {
    ($($node_type:ty),* $(,)?) => {
        $(
            impl Styleable for RawHaalkaEl<$node_type> {
                fn update_style(mut self, updater: impl FnOnce(&mut Style)) -> Self {
                    let style = &mut self.0.as_mut().unwrap().raw_node.style;
                    updater(style);
                    self
                }
            }
        )*
    };
}

impl_styleable_for_nodes!(
    NodeBundle,
    ImageBundle,
    AtlasImageBundle,
    TextBundle,
    ButtonBundle,
    // MaterialNodeBundle<impl UiMaterial>,  // TODO
);

pub struct RawHaalkaEl<NodeType>(pub Option<NodeBuilder<NodeType>>);

impl<NodeType: Bundle> From<NodeType> for RawHaalkaEl<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self(Some(NodeBuilder::from(node_bundle)))
    }
}

impl<NodeType: Bundle> RawEl for RawHaalkaEl<NodeType> {
    type NodeType = NodeType;

    fn new_dummy() -> Self {
        Self(None)
    }

    fn update_node_builder(mut self, updater: impl FnOnce(NodeBuilder<Self::NodeType>) -> NodeBuilder<Self::NodeType>) -> Self {
        self.0 = Some(updater(self.0.unwrap()));
        self
    }

    fn into_node_builder(self) -> NodeBuilder<Self::NodeType> {
        self.0.unwrap()
    }
}

pub trait RawElWrapper: Sized {
    type RawEl: RawEl;

    fn raw_el_mut(&mut self) -> &mut Self::RawEl;

    fn update_raw_el(mut self, updater: impl FnOnce(Self::RawEl) -> Self::RawEl) -> Self {
        let raw_el = mem::replace(self.raw_el_mut(), RawEl::new_dummy());
        mem::swap(self.raw_el_mut(), &mut updater(raw_el));
        self
    }

    fn into_raw_el(mut self) -> Self::RawEl {
        mem::replace(self.raw_el_mut(), RawEl::new_dummy())
    }
}

impl<NodeType: Bundle> Element for RawHaalkaEl<NodeType> {
    type NodeType = NodeType;

    fn into_raw(self) -> Self {
        self
    }
}

pub struct El<RE: RawEl>(pub RE);

impl<NodeType: Bundle> From<NodeType> for El<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    fn from(node_bundle: NodeType) -> Self {
        let raw_el = RawHaalkaEl::from(node_bundle);
        let raw_el = raw_el.update_style(|style| {
            style.display = Display::Flex;
        });
        Self(raw_el)
    }
}

impl<NodeType: Bundle + Default> El<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<RE: RawEl> RawElWrapper for El<RE> {
    type RawEl = RE;

    fn raw_el_mut(&mut self) -> &mut Self::RawEl {
        &mut self.0
    }
}

pub trait MouseEventAware: RawElWrapper + Sized {
    fn on_hovered_change(self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.insert(Hoverable(Box::new(handler)))))
    }

    fn on_press(self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.update_raw_el(|raw_el| raw_el.update_node_builder(|node_builder| node_builder.insert(Pressable(Box::new(handler)))))
    }
}

impl<RE: RawEl> MouseEventAware for El<RE> {}

pub trait Spawnable: RawElWrapper + Sized {
    fn spawn(self, world: &mut World) -> Entity {
        self.into_raw_el().spawn(world)
    }
}

impl<RE: RawEl> Spawnable for El<RE> {}

pub trait ComponentSignalable: RawElWrapper + Sized {
    fn component_signal(self, component_signal: impl Signal<Item = impl Component> + 'static + Send + Sync) -> Self {
        self.update_raw_el(|raw_el| raw_el.component_signal(component_signal))
    }
}

impl<RE: RawEl> ComponentSignalable for El<RE> {}

impl<NodeType: Bundle> El<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn child(mut self, child_option: impl IntoOptionElement) -> Self {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn child_signal(mut self, child_option: impl Signal<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn children<I: IntoIterator<Item = impl IntoOptionElement> + 'static + Send + Sync>(mut self, children: I) -> Self
    where I::IntoIter: Send + Sync
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn children_signal_vec(mut self, children_signal_vec: impl SignalVec<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

pub struct Column<RE: RawEl>(pub RE);  // TODO: impl Element like api so the inner raw el's don't need to be managed

impl<NodeType: Bundle> From<NodeType> for Column<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    fn from(node_bundle: NodeType) -> Self {
        let raw_el = RawHaalkaEl::from(node_bundle);
        let raw_el = raw_el.update_style(|style| {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Column;
        });
        Self(raw_el)
    }
}

impl<NodeType: Bundle + Default> Column<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<RE: RawEl> RawElWrapper for Column<RE> {
    type RawEl = RE;

    fn raw_el_mut(&mut self) -> &mut Self::RawEl {
        &mut self.0
    }
}

impl<NodeType: Bundle> Column<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn item(mut self, child_option: impl IntoOptionElement) -> Self {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn item_signal(mut self, child_option: impl Signal<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn items<I: IntoIterator<Item = impl IntoOptionElement> + 'static + Send + Sync>(mut self, children: I) -> Self
    where I::IntoIter: Send + Sync
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn items_signal_vec(mut self, children_signal_vec: impl SignalVec<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

pub struct Row<RE: RawEl>(RE);

impl<NodeType: Bundle> From<NodeType> for Row<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    fn from(node_bundle: NodeType) -> Self {
        let raw_el = RawHaalkaEl::from(node_bundle);
        let raw_el = raw_el.update_style(|style| {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Row;
        });
        Self(raw_el)
    }
}

impl<NodeType: Bundle + Default> Row<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> Row<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn item(mut self, child_option: impl IntoOptionElement) -> Self {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn item_signal(mut self, child_option: impl Signal<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn items<I: IntoIterator<Item = impl IntoOptionElement> + 'static + Send + Sync>(mut self, children: I) -> Self
    where I::IntoIter: Send + Sync
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn items_signal_vec(mut self, children_signal_vec: impl SignalVec<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

pub struct Stack<RE: RawEl>(RE);

impl<NodeType: Bundle + Styleable> From<NodeType> for Stack<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    fn from(node_bundle: NodeType) -> Self {
        let mut raw_el = RawHaalkaEl::from(node_bundle);
        raw_el = raw_el.update_style(|style| {
            style.display = Display::Grid;
            style.grid_auto_columns = vec![GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto)];
            style.grid_auto_rows = vec![GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto)];
        });
        Self(raw_el)
    }
}

impl<NodeType: Bundle + Styleable + Default> Stack<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle + Styleable> Stack<RawHaalkaEl<NodeType>> where RawHaalkaEl<NodeType>: Styleable {
    pub fn layer(mut self, child_option: impl IntoOptionElement) -> Self {
        self.0 = self.0.child(child_option);
        self
    }

    pub fn layer_signal(mut self, child_option: impl Signal<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.child_signal(child_option);
        self
    }

    pub fn layers<I: IntoIterator<Item = impl IntoOptionElement> + 'static + Send + Sync>(mut self, children: I) -> Self
    where I::IntoIter: Send + Sync
    {
        self.0 = self.0.children(children);
        self
    }

    pub fn layers_signal_vec(mut self, children_signal_vec: impl SignalVec<Item = impl IntoOptionElement + Send + 'static> + 'static + Send + Sync) -> Self {
        self.0 = self.0.children_signal_vec(children_signal_vec);
        self
    }
}

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
