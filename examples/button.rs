use bevy::prelude::*;
use haalka::*;
use futures_signals::{signal::{Mutable, SignalExt}, map_ref};


fn main() {
    App::new()
        .add_plugins((DefaultPlugins, HaalkaPlugin))
        .add_systems(Startup, (setup, spawn_ui_root))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn button(font: Handle<Font>) -> impl Element {
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
    El::<ButtonBundle>::new()
    .with_style(|style| {
        style.width = Val::Px(150.0);
        style.height = Val::Px(65.);
        style.border = UiRect::all(Val::Px(5.0));
    })
    .align_content(vec![Align::CenterX, Align::CenterY])
    .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    .on_pressed_change(move |is_pressed| pressed.set_neq(is_pressed))
    .border_color_signal(border_color_signal)
    .background_color_signal(background_color_signal)
    .child({
        let text_signal = {
            pressed_hovered_broadcaster.signal()
            .map(|(pressed, hovered)| {
                if pressed {
                    "Press"
                } else if hovered {
                    "Hover"
                } else {
                    "Button"
                }
                .to_string()
            })
            .map(move |string| {
                let text_style = {
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    }
                };
                Text::from_section(string, text_style)
            })
        };
        El::<TextBundle>::new().text_signal(text_signal)
    })
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_ui_root(world: &mut World) {
    El::<NodeBundle>::new()
    .with_style(|style| {
        style.width = Val::Percent(100.0);
        style.height = Val::Percent(100.0);
    })
    .align_content(vec![Align::CenterX, Align::CenterY])
    .child(button(world.resource::<AssetServer>().load("fonts/FiraSans-Bold.ttf")))
    .spawn(world);
}
