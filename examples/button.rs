//! Simple button, port of <https://github.com/bevyengine/bevy/blob/main/examples/ui/button.rs>.

mod utils;
use bevy_ui::Pressed;
use utils::*;

use bevy::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
                },
                camera,
            ),
        )
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

// #[track_caller]
fn button() -> impl Element {
    let lazy_entity = LazyEntity::new();

    let pressed_hovered_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
        // TODO: can't use .has_component because Pressed is not Clone
        // .has_component::<Pressed>()
        .map(|In(entity), presseds: Query<&Pressed>| presseds.contains(entity))
        .dedupe()
        .combine(
            SignalBuilder::from_lazy_entity(lazy_entity.clone())
                .has_component::<Hovered>()
                .dedupe(),
        )
        .dedupe();

    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(150.0);
            node.height = Val::Px(65.);
            node.border = UiRect::all(Val::Px(5.0));
        })
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .align_content(Align::center())
        .border_radius(BorderRadius::MAX)
        .with_builder(|builder| builder.lazy_entity(lazy_entity.clone()))
        .border_color_signal(
            pressed_hovered_signal
                .clone()
                .map_in(|(pressed, hovered)| {
                    if pressed {
                        bevy::color::palettes::basic::RED.into()
                    } else if hovered {
                        Color::WHITE
                    } else {
                        Color::BLACK
                    }
                })
                .map_in(BorderColor::all)
                .map_in(Some),
        )
        .background_color_signal(
            pressed_hovered_signal
                .clone()
                .map_in(|(pressed, hovered)| {
                    if pressed {
                        PRESSED_BUTTON
                    } else if hovered {
                        HOVERED_BUTTON
                    } else {
                        NORMAL_BUTTON
                    }
                })
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .child(
            El::<Text>::new()
                .text_font(TextFont {
                    font_size: 33.0,
                    ..default()
                })
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text_signal(
                    pressed_hovered_signal
                        .map_in(|(pressed, hovered)| {
                            if pressed {
                                "Press"
                            } else if hovered {
                                "Hover"
                            } else {
                                "Button"
                            }
                        })
                        .map_in(Text::new)
                        .map_in(Some),
                ),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(button())
}
