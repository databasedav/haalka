use std::{sync::OnceLock, future::Future, marker::PhantomData, mem, time::Duration};
use bevy::{
    prelude::*,
    ecs::system::{CommandQueue, EntityCommand, Command},
    tasks::{AsyncComputeTaskPool, Task, TaskPool}, core_pipeline::core_2d::graph::node,
};
use futures_signals::{signal::{Mutable, Signal, SignalExt, always}, map_ref, signal_vec::{SignalVec, SignalVecExt, VecDiff, MutableVec}};
use bevy_async_ecs::*;
use enclose::enclose as clone;
use async_io::Timer;


const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

static ASYNC_WORLD: OnceLock<AsyncWorld> = OnceLock::new();

fn async_world() -> &'static AsyncWorld {
    ASYNC_WORLD.get().expect("expected AsyncWorld to be initialized")
}

#[derive(Component)]
struct Hoverable(Box<dyn FnMut(bool) + 'static + Send + Sync>);

#[derive(Component)]
struct Pressable(Box<dyn FnMut(bool) + 'static + Send + Sync>);

#[derive(Component)]
struct TaskHolder(Vec<Task<()>>);

#[derive(Default)]
struct Node<NodeType> {
    raw_node: NodeType,
    on_spawns: Vec<Box<dyn FnOnce(&mut World, Entity) + Send + Sync>>,
    task_wrappers: Vec<Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send + Sync>>,
    contiguous_child_block_populations: MutableVec<usize>,
    child_block_inserted: MutableVec<bool>,
    node_type: PhantomData<NodeType>,
}

impl<T: Bundle + Default> From<T> for Node<T> {
    fn from(node_bundle: T) -> Self {
        Node {
            raw_node: node_bundle,
            ..default()
        }
    }
}

fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}

async fn sync_component<T: Component>(async_world: AsyncWorld, entity: Entity, component_signal: impl Signal<Item = T> + 'static + Send + Sync) {
    // TODO: need partial_eq derivations for all the node related components to minimize updates
    component_signal.for_each(|value| {
        spawn(clone!((async_world) async move {
            async_world.apply(move |world: &mut World| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    entity.insert(value);
                }
            }).await;
        }))
    }).await;
}

fn sync_component_task_wrapper<T: Component>(component_signal: impl Signal<Item = T> + 'static + Send + Sync) -> Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send + Sync> {
    Box::new(|async_world: AsyncWorld, entity: Entity| {
        spawn(sync_component(async_world, entity, component_signal))
    })
}

// TODO: macro to only impl for the node bundles
impl<NodeType: Default + Bundle> Node<NodeType> {
    fn on_spawn(mut self, on_spawn: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) -> Self {
        self.on_spawns.push(Box::new(on_spawn));
        self
    }

    fn insert<T: Bundle>(mut self, bundle: T) -> Self {
        self.on_spawn(|world: &mut World, entity: Entity| {
            if let Some(mut entity) = world.get_entity_mut(entity) {
                entity.insert(bundle);
            }
        })
    }

    fn border_color(mut self, border_color_signal: impl Signal<Item = BorderColor> + 'static + Send + Sync) -> Self {
        self.task_wrappers.push(sync_component_task_wrapper(border_color_signal));
        self
    }

    fn background_color(mut self, background_color_signal: impl Signal<Item = BackgroundColor> + 'static + Send + Sync) -> Self {
        self.task_wrappers.push(Box::new(sync_component_task_wrapper(background_color_signal)));
        self
    }

    fn z_index(mut self, z_index_signal: impl Signal<Item = ZIndex> + 'static + Send + Sync) -> Self {
        self.task_wrappers.push(Box::new(sync_component_task_wrapper(z_index_signal)));
        self
    }

    // TODO: list out limitations; limitation: if multiple children are added to entity, they must be registered thru this abstraction because of the way siblings are tracked
    fn child<ChildNodeType: Bundle + Default>(mut self, child: Node<ChildNodeType>) -> Self {
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

    fn child_signal<ChildNodeType: Bundle + Default>(mut self, child_option: impl Signal<Item = impl Into<Option<Node<ChildNodeType>>> + Send + 'static> + 'static + Send + Sync) -> Self {
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
                                if let Some(existing_child) = mutable_take(&existing_child_option) {
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
                                if let Some(existing_child) = mutable_take(&existing_child_option) {
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

    fn children<ChildNodeType: Bundle + Default>(mut self, children: impl IntoIterator<Item = impl Into<Option<Node<ChildNodeType>>>> + 'static + Send + Sync) -> Self {
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
                        if let Some(child) = child.into() {
                            children_entities.push(child.spawn(world));
                        }
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

    fn children_signal_vec<ChildNodeType: Bundle + Default>(mut self, children_signal_vec: impl SignalVec<Item = impl Into<Option<Node<ChildNodeType>>> + Send + 'static> + 'static + Send + Sync) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(0);
        self.child_block_inserted.lock_mut().push(false);
        let child_block_inserted = self.child_block_inserted.clone();
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) {
                let children_entities = MutableVec::default();
                children_signal_vec.filter_map(|child_option| child_option.into())
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

    fn spawn(self, world: &mut World) -> Entity {
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

impl Node<ButtonBundle> {
    fn on_hovered_change(mut self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.insert(Hoverable(Box::new(handler)))
    }

    fn on_press(mut self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.insert(Pressable(Box::new(handler)))
    }
}

impl Node<TextBundle> {
    fn text(mut self, text_signal: impl Signal<Item = Text> + 'static + Send + Sync) -> Self {
        self.task_wrappers.push(sync_component_task_wrapper(text_signal));
        self
    }
}

// TODO: separate utilites like moonzoon (.take() copy)
fn mutable_take<T: Default>(mutable: &Mutable<T>) -> T {
    mem::take(&mut *mutable.lock_mut())
}

async fn wait_until_child_block_inserted(block: usize, child_block_inserted: &MutableVec<bool>) {
    child_block_inserted.signal_vec().to_signal_map(|last_child_block_inserted| last_child_block_inserted[block]).wait_for(true).await;
}

fn button(text: String) -> Node<ButtonBundle> {
    let hovered = Mutable::new(false);
    let pressed = Mutable::new(false);
    let mut button_node = Node::from(
        ButtonBundle {
            style: Style {
                width: Val::Px(190.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        }
    );
    let pressed_hovered = map_ref! {
        let pressed = pressed.signal(),
        let hovered = hovered.signal() => {
            (*pressed, *hovered)
        }
    }.broadcast();
    let border_color = {
        pressed_hovered.signal()
        .map(|(pressed, hovered)| {
            if pressed {
                Color::RED
            } else if hovered {
                Color::WHITE
            } else {
                Color::BLACK
            }
        })
        .map(BorderColor)
    };
    let background_color = {
        pressed_hovered.signal()
        .map(|(pressed, hovered)| {
            if pressed {
                PRESSED_BUTTON
            } else if hovered {
                HOVERED_BUTTON
            } else {
                NORMAL_BUTTON
            }
        })
        .map(BackgroundColor)
    };
    let text = {
        pressed_hovered.signal()
        .map(move |(pressed, hovered)| {
            if pressed {
                "Press".to_string()
            } else if hovered {
                "Hover".to_string()
            } else {
                text.clone()
            }
        })
    };
    button_node = {
        button_node
        .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
        .background_color(background_color)
        .border_color(border_color)
        .on_press(move |is_pressed| pressed.set_neq(is_pressed))
    };
    let text_style = {
        TextStyle {
            font_size: 40.0,
            color: Color::rgb(0.9, 0.9, 0.9),
            ..default()
        }
    };
    let text_node = TextBundle::from_section(String::new(), text_style.clone());
    button_node
    .child(
        Node::from(text_node)
        .text(text.map(move |text| Text::from_section(text, text_style.clone())))
    )
}

#[derive(Component, Default)]
struct MutableHolder {
    mutable_bool: Mutable<bool>,
    mutable_number: Mutable<usize>,
    mutable_string: Mutable<String>,
    mutable_bool_vec: MutableVec<bool>,
    mutable_number_vec: MutableVec<usize>,
    mutable_string_vec: MutableVec<String>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, AsyncEcsPlugin))
        .add_event::<MutableEvent>()
        .add_systems(Startup, |world: &mut World| {
            let mut root_node = Node::from(
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                }
            );
            let show = Mutable::new(true);
            spawn(clone!((show) async move {
                loop {
                    Timer::after(Duration::from_secs(1)).await;
                    show.set(!show.get());
                }
            })).detach();

            let mutable_holder = MutableHolder::default();
            let buttons = MutableVec::new_with_values(vec![4, 5, 6]);
            spawn(clone!((buttons) async move {
                let mut i = 0;
                loop {
                    Timer::after(Duration::from_secs(1)).await;
                    if i == 0 {
                        buttons.lock_mut().push(7);
                        i += 1;
                    } else {
                        buttons.lock_mut().pop();
                        i -= 1;
                    }
                }
            })).detach();
            root_node = {
                root_node
                // .children(vec![
                //     button("button -2".to_string()),
                //     button("button -1".to_string()),
                //     button("button 0".to_string()),
                // ])
                .child(button("button 1".to_string()))
                .child_signal(
                    mutable_holder.mutable_bool.signal().dedupe()
                    // show.signal().dedupe()
                    .map(move |show| {
                        if show {
                            Some(button("button 2".to_string()))
                        } else {
                            None
                        }
                    })
                )
                .child(button("button 3".to_string()))
                .children_signal_vec(buttons.signal_vec().map(|n| button(format!("button {}", n))))
                .insert(mutable_holder)
            };
            root_node.spawn(world);
		})
        .add_systems(Startup, (
            setup,
            // init_async_world
        ))
        .add_systems(Update, (
            hoverable_system,
            pressable_system,
            mutable_updater_system,
            mutable_event_listener,
        ))
        .run();
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

#[derive(Event)]
struct MutableEvent(bool);

fn mutable_updater_system(
    mut interaction_query: Query<(&Interaction, &mut Pressable)>, /* Changed<Interaction>>, */  // TODO: explain the bug that occurs when using Changed
    mut mutable_events: EventWriter<MutableEvent>,
) {
    for (interaction, mut pressable) in &mut interaction_query {
        if matches!(interaction, Interaction::Pressed) {
            mutable_events.send(MutableEvent(true));
            return;
        }
    }
    // println!("not pressed");
    mutable_events.send(MutableEvent(false));
}

fn mutable_event_listener(
    mut mutable_events: EventReader<MutableEvent>,
    mut mutable_holder_query: Query<&mut MutableHolder>,
) {
    for mutable_event in mutable_events.read() {
        for mut mutable_holder in &mut mutable_holder_query {
            mutable_holder.mutable_bool.set_neq(mutable_event.0);
        }
    }
}

fn init_async_world(world: &mut World) {
    ASYNC_WORLD.set(AsyncWorld::from_world(world)).unwrap();
    AsyncComputeTaskPool::get_or_init(|| {
        let task_pool = TaskPool::default();
        task_pool.with_local_executor(|_| {
            ASYNC_WORLD.set(AsyncWorld::from_world(world)).unwrap();
        });
        task_pool
    });
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
