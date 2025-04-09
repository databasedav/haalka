//! Simple button, port of <https://github.com/bevyengine/bevy/blob/main/examples/ui/button.rs>.

mod utils;
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

fn button() -> impl Element {
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    let pressed_hovered_broadcaster =
        map_ref!(pressed_signal, hovered_signal => (*pressed_signal, *hovered_signal)).broadcast();
    let border_color_signal = {
        pressed_hovered_broadcaster
            .signal()
            .map(|(pressed, hovered)| {
                if pressed {
                    bevy::color::palettes::basic::RED.into()
                } else if hovered {
                    Color::WHITE
                } else {
                    Color::BLACK
                }
            })
            .map(BorderColor)
    };
    let background_color_signal = {
        pressed_hovered_broadcaster
            .signal()
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
    El::<Node>::new()
        .width(Val::Px(150.0))
        .height(Val::Px(65.))
        .with_node(|mut node| node.border = UiRect::all(Val::Px(5.0)))
        .align_content(Align::center())
        .border_color_signal(border_color_signal)
        .background_color_signal(background_color_signal)
        .border_radius(BorderRadius::MAX)
        .hovered_sync(hovered)
        .pressed_sync(pressed)
        .child(
            El::<Text>::new()
                .text_font(TextFont {
                    font_size: 33.0,
                    ..default()
                })
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text_signal(
                    pressed_hovered_broadcaster
                        .signal()
                        .map(|(pressed, hovered)| {
                            if pressed {
                                "Press"
                            } else if hovered {
                                "Hover"
                            } else {
                                "Button"
                            }
                        })
                        .map(Text::new),
                ),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(button())
}
