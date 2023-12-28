use std::{sync::OnceLock, future::Future, marker::PhantomData, mem};
use bevy::{
    prelude::*,
    ecs::system::{CommandQueue, EntityCommand, Command},
    tasks::{AsyncComputeTaskPool, Task, TaskPool},
};
use futures_signals::{signal::{Mutable, Signal, SignalExt, always}, map_ref};
use bevy_async_ecs::*;
use enclose::enclose as clone;


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
struct BorderColorSignal(Mutable<Color>);

#[derive(Component)]
struct BackgroundColorSignal(Mutable<Color>);

#[derive(Component)]
struct TextSignal(Mutable<String>);

#[derive(Component)]
struct TaskHolder(Vec<Task<()>>);

#[derive(Default)]
struct Node<NodeType> {
    raw_node: NodeType,
    hoverable: Option<Hoverable>,
    pressable: Option<Pressable>,
    node_type: PhantomData<NodeType>,
    task_wrappers: Vec<Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send + Sync>>,
}

impl<T: Bundle + Default> From<T> for Node<T> {
    fn from(node_bundle: T) -> Self {
        Node {
            raw_node: node_bundle,
            ..default()
        }
    }
}

enum Styles {
    BackgroundColor,
    BorderColor,
}

fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}

async fn sync_component<T: Component>(async_world: AsyncWorld, entity: Entity, component_signal: impl Signal<Item = T> + 'static + Send + Sync) {
    // TODO: need partial_eq derivations for all the node related components to minimize updates
    component_signal.for_each(|value| {
        spawn(clone!((async_world) async move { async_world.entity(entity).insert(value).await; }))
    }).await;
}

fn sync_component_task_wrapper<T: Component>(component_signal: impl Signal<Item = T> + 'static + Send + Sync) -> Box<dyn FnOnce(AsyncWorld, Entity) -> Task<()> + Send + Sync> {
    Box::new(|async_world: AsyncWorld, entity: Entity| {
        spawn(sync_component(async_world, entity, component_signal))
    })
}

// TODO: macro to only impl for the node bundles
impl<NodeType: Default + Bundle> Node<NodeType> {
    fn new() -> Self {
        default()
    }

    fn on_hovered_change(mut self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.hoverable = Some(Hoverable(Box::new(handler)));
        self
    }

    fn on_press(mut self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.pressable = Some(Pressable(Box::new(handler)));
        self
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

    fn child<ChildNodeType: Bundle + Default>(mut self, child_option: impl Signal<Item = impl Into<Option<Node<ChildNodeType>>> + Send + 'static> + 'static + Send + Sync) -> Self {
        let task_wrapper = |async_world: AsyncWorld, entity: Entity| {
            spawn(clone!((async_world, entity => parent) {
                let existing_child_option = Mutable::new(None);
                child_option.for_each(move |child_option| {
                    // TODO: should be like this after https://github.com/dlom/bevy-async-ecs/issues/2
                    // spawn(async_world().entity(entity).insert(border_color)).detach();
                    spawn(clone!((async_world, existing_child_option) async move {
                        if let Some(child) = child_option.into() {
                            async_world.apply(move |world: &mut World| {
                                if let Some(existing_child) = mem::take(&mut *existing_child_option.lock_mut()) {
                                    world.entity_mut(existing_child).despawn_recursive();  // removes from parent
                                }
                                let child_entity = child.spawn(world);
                                let mut parent = world.entity_mut(parent);
                                parent.add_child(child_entity);
                                existing_child_option.set(Some(child_entity));
                            }).await;
                        } else {
                            async_world.apply(move |world: &mut World| {
                                let mut parent = world.entity_mut(parent);
                                if let Some(existing_child) = mem::take(&mut *existing_child_option.lock_mut()) {
                                    parent.remove_children(&[existing_child]);
                                    world.entity_mut(existing_child).despawn_recursive();
                                }
                            }).await;
                        }
                    })).detach();
                    async {}
                })
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    fn spawn(self, world: &mut World) -> Entity {
        let async_world = AsyncWorld::from_world(world);
        let mut entity_mut = world.spawn(self.raw_node);
        if let Some(hoverable) = self.hoverable {
            entity_mut.insert(hoverable);
        }
        if let Some(pressable) = self.pressable {
            entity_mut.insert(pressable);
        }
        let id = entity_mut.id();
        if !self.task_wrappers.is_empty() {
            let mut tasks = vec![];
            for task_wrapper in self.task_wrappers {
                tasks.push(task_wrapper(async_world.clone(), id));
            }
            entity_mut.insert(TaskHolder(tasks));
        }
        id
    }
}

impl Node<TextBundle> {
    fn text(mut self, text_signal: impl Signal<Item = String> + 'static + Send + Sync) -> Self {
        self.task_wrappers.push(sync_component_task_wrapper(text_signal.map(|text| Text::from_section(text, default()))));
        self
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, AsyncEcsPlugin))
        .add_systems(Startup, |world: &mut World| {
            let hovered = Mutable::new(false);
            let pressed = Mutable::new(false);
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
                .map(|(pressed, hovered)| {
                    if pressed {
                        "Press".to_string()
                    } else if hovered {
                        "Hover".to_string()
                    } else {
                        "Button".to_string()
                    }
                })
            };
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
            let mut button_node = Node::from(
                ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                }
            );
            button_node = {
                button_node
                .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
                .on_press(move |is_pressed| pressed.set_neq(is_pressed))
                .background_color(background_color)
                .border_color(border_color)
            };
            let text_node = {
                TextBundle::from_section(
                    String::new(),
                    TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                )
            };
            root_node = {
                root_node
                .child(always(Some(button_node.child(always(Some(Node::from(text_node).text(text)))))))
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
        ))
        .run();
}

fn border_color_updater(mut border_color_query: Query<(&mut BorderColor, &BorderColorSignal)>) {
    for (mut border_color, border_color_signal) in &mut border_color_query {
        border_color.0 = border_color_signal.0.get();
    }
}

fn background_color_updater(mut background_color_query: Query<(&mut BackgroundColor, &BackgroundColorSignal)>) {
    for (mut background_color, background_color_signal) in &mut background_color_query {
        background_color.0 = background_color_signal.0.get();
    }
}

fn text_updater(mut text_query: Query<(&mut Text, &TextSignal)>) {
    for (mut text, text_signal) in &mut text_query {
        text.sections[0].value = text_signal.0.get_cloned();
    }
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
