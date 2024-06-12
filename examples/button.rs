use bevy::prelude::*;
use haalka::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin,
        ))
        .add_systems(Startup, (setup, spawn_ui_root))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn button(font: Handle<Font>) -> impl Element {
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    let pressed_hovered_broadcaster =
        map_ref!(pressed_signal, hovered_signal => (*pressed_signal, *hovered_signal)).broadcast();
    let border_color_signal = {
        pressed_hovered_broadcaster
            .signal()
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
    El::<NodeBundle>::new()
        .width(Val::Px(150.0))
        .height(Val::Px(65.))
        .with_style(|style| style.border = UiRect::all(Val::Px(5.0)))
        .align_content(Align::center())
        .border_color_signal(border_color_signal)
        .background_color_signal(background_color_signal)
        .hovered_sync(hovered)
        .pressed_sync(pressed)
        .child(
            El::<TextBundle>::new().text_signal(
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
                    .map(move |string| {
                        Text::from_section(
                            string,
                            TextStyle {
                                font: font.clone(),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        )
                    }),
            ),
        )
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(button(world.resource::<AssetServer>().load("fonts/FiraSans-Bold.ttf")))
        .spawn(world);
}
