use bevy::{
    prelude::*,
    ecs::system::CommandQueue,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_signals::{signal::{Mutable, Signal, SignalExt}, map_ref};
use bevy_async_ecs::*;
use enclose::enclose as clone;


const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

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

struct El {
    hoverable: Option<Hoverable>,
    pressable: Option<Pressable>,
    text_signal: Option<TextSignal>,
    background_color_signal: Option<BackgroundColorSignal>,
    border_color_signal: Option<BorderColorSignal>,
    tasks: Vec<Task<()>>,
}

enum Styles {
    BackgroundColor,
    BorderColor,
}

impl El {
    fn new() -> Self {
        Self {
            hoverable: None,
            pressable: None,
            text_signal: None,
            background_color_signal: None,
            border_color_signal: None,
            tasks: Vec::new(),
        }
    }

    fn on_hovered_change(mut self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.hoverable = Some(Hoverable(Box::new(handler)));
        self
    }

    fn on_press(mut self, handler: impl FnMut(bool) + 'static + Send + Sync) -> Self {
        self.pressable = Some(Pressable(Box::new(handler)));
        self
    }

    fn border_color(mut self, signal: impl Signal<Item = Color> + 'static + Send + Sync) -> Self {
        let border_color = Mutable::new(Color::BLACK);
        let task = AsyncComputeTaskPool::get().spawn(clone!((border_color) async move {
            signal.for_each(|c| {
                border_color.set_neq(c);
                async {}
            }).await;
        }));
        self.border_color_signal = Some(BorderColorSignal(border_color));
        self.tasks.push(task);
        self
    }

    fn background_color(mut self, signal: impl Signal<Item = Color> + 'static + Send + Sync) -> Self {
        let background_color = Mutable::new(NORMAL_BUTTON);
        let task = AsyncComputeTaskPool::get().spawn(clone!((background_color) async move {
            signal.for_each(|c| {
                background_color.set_neq(c);
                async {}
            }).await;
        }));
        self.background_color_signal = Some(BackgroundColorSignal(background_color));
        self.tasks.push(task);
        self
    }

    fn text(mut self, signal: impl Signal<Item = String> + 'static + Send + Sync) -> Self {
        let text = Mutable::new(String::new());
        let task = AsyncComputeTaskPool::get().spawn(clone!((text) async move {
            signal.for_each(|t| {
                text.set_neq(t);
                async {}
            }).await;
        }));
        self.text_signal = Some(TextSignal(text));
        self.tasks.push(task);
        self
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, AsyncEcsPlugin))
        .add_systems(Startup, |world: &mut World| {
            let hovered = Mutable::new(false);
            let pressed = Mutable::new(false);
            let border_color = map_ref! {
                let pressed = pressed.signal(),
                let hovered = hovered.signal() => {
                    if *pressed {
                        Color::RED
                    } else if *hovered {
                        Color::WHITE
                    } else {
                        Color::BLACK
                    }
                }
            };
            let background_color = map_ref! {
                let pressed = pressed.signal(),
                let hovered = hovered.signal() => {
                    if *pressed {
                        PRESSED_BUTTON
                    } else if *hovered {
                        HOVERED_BUTTON
                    } else {
                        NORMAL_BUTTON
                    }
                }
            };
            let text = map_ref! {
                let pressed = pressed.signal(),
                let hovered = hovered.signal() => {
                    if *pressed {
                        "Press".to_string()
                    } else if *hovered {
                        "Hover".to_string()
                    } else {
                        "Button".to_string()
                    }
                }
            };
            let el = {
                El::new()
                .on_hovered_change(clone!((hovered) move |is_hovered| hovered.set_neq(is_hovered)))
                .on_press(clone!((pressed) move |is_pressed| pressed.set_neq(is_pressed)))
                .border_color(border_color)
                .background_color(background_color)
                .text(text)
            };

            let entity = world.spawn(
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                }
            ).id();
			let async_world = AsyncWorld::from_world(world);
			let fut = async move {
                let mut command_queue = CommandQueue::default();
                command_queue.push(move |world: &mut World| {
                    world.entity_mut(entity)
                    .with_children(|parent| {
                        let mut entity = parent.spawn((
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
                            },
                        ));
                        if let Some(hoverable) = el.hoverable {
                            entity.insert(hoverable);
                        }
                        if let Some(pressable) = el.pressable {
                            entity.insert(pressable);
                        }
                        if let Some(background_color_signal) = el.background_color_signal {
                            entity.insert(background_color_signal);
                        }
                        if let Some(border_color_signal) = el.border_color_signal {
                            entity.insert(border_color_signal);
                        }
                        if let Some(text_signal) = el.text_signal {
                            entity.with_children(|parent| {
                                parent.spawn(
                                    TextBundle::from_section(
                                        String::new(),
                                        TextStyle {
                                            font_size: 40.0,
                                            color: Color::rgb(0.9, 0.9, 0.9),
                                            ..default()
                                        },
                                    )
                                )
                                .insert(text_signal);
                            });
                        }
                        entity.insert(TaskHolder(el.tasks));
                    });
                });
                async_world.entity(entity).sender().send_queue(command_queue).await;
			};
            AsyncComputeTaskPool::get().spawn(fut).detach();
		})
        .add_systems(Startup, setup)
        .add_systems(Update, (
            hoverable_system,
            pressable_system,
            border_color_updater,
            background_color_updater,
            text_updater,
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
