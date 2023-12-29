use bevy::prelude::*;
use haalka::{*, Node};
use futures_signals::{signal::{Mutable, SignalExt}, map_ref};


fn main() {
    App::new()
        .add_plugins((DefaultPlugins, HaalkaPlugin))
        .add_systems(Startup, (setup, insert_ui_root))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn button(font: Handle<Font>) -> Node<ButtonBundle> {
    let hovered = Mutable::new(false);
    let pressed = Mutable::new(false);
    let pressed_hovered_broadcaster = map_ref! {
        let pressed = pressed.signal(),
        let hovered = hovered.signal() => {
            (*pressed, *hovered)
        }
    }.broadcast();
    let border_color_signal = {
        pressed_hovered_broadcaster.signal()
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
    let background_color_signal = {
        pressed_hovered_broadcaster.signal()
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
    let button_node = Node::from(ButtonBundle {
        style: Style {
            width: Val::Px(150.0),
            height: Val::Px(65.0),
            border: UiRect::all(Val::Px(5.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        border_color: BorderColor(Color::BLACK),
        background_color: NORMAL_BUTTON.into(),
        ..default()
    });
    button_node
    .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    .on_press(move |is_pressed| pressed.set_neq(is_pressed))
    .border_color(border_color_signal)
    .background_color(background_color_signal)
    .child({
        let text_node = Node::from(TextBundle::from_section(
            "Button",
            TextStyle {
                font,
                font_size: 40.0,
                color: Color::rgb(0.9, 0.9, 0.9),
            },
        ));
        let text_signal = {
            pressed_hovered_broadcaster.signal()
            .map(move |(pressed, hovered)| {
                if pressed {
                    "Press"
                } else if hovered {
                    "Hover"
                } else {
                    "Button"
                }
                .to_string()
            })
            .map(|string| {
                let text_style = {
                    TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    }
                };
                Text::from_section(string, text_style)
            })
        };
        text_node
        .text(text_signal)
    })
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn insert_ui_root(world: &mut World) {
    let root_node = Node::from(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ..default()
    });
    root_node
    .child(button(world.resource::<AssetServer>().load("fonts/FiraSans-Bold.ttf")))
    .spawn(world);
}
