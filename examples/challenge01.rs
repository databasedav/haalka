// Main menu with sub menus for audio and graphics.
// Simple buttons for option selection.
// Slider for volume.
// Dropdown for graphics quality (low/medium/high).
// Navigation possible with mouse, keyboard and controller.
//     Mouse: Separate styles for hover and press.
//     Keyboard/Controller: Separate styles for currently focused element.

use bevy::prelude::*;
use haalka::*;
use futures_signals::{signal::{Mutable, SignalExt}, map_ref};
use futures_signals_ext::*;


fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin
        ))
        .add_systems(Startup, (setup, insert_ui_root))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

#[derive(Clone, Copy, PartialEq)]
enum SubMenu {
    Audio,
    Graphics,
}

fn button(sub_menu: SubMenu, show_sub_menu: Mutable<Option<SubMenu>>) -> impl Element {
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
    let button_node = El::from(ButtonBundle {
        style: Style {
            width: Val::Px(180.0),
            height: Val::Px(65.),
            border: UiRect::all(Val::Px(5.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    });
    button_node
    .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    .on_press(move |is_pressed| {
        if is_pressed { show_sub_menu.set_neq(Some(sub_menu)) }
        pressed.set_neq(is_pressed);
    })
    .border_color_signal(border_color_signal)
    .background_color_signal(background_color_signal)
    .child({
        let text_style = {
            TextStyle {
                font_size: 40.0,
                color: TEXT_COLOR,
                ..default()
            }
        };
        El::from(
            TextBundle {
                text: Text::from_section(match sub_menu { SubMenu::Audio => "audio", SubMenu::Graphics => "graphics" }, text_style),
                ..default()
            }
        )
    })
}

fn menu_base(sides: f32) -> Column<NodeBundle> {
    Column::from(NodeBundle {
        style: Style {
            width: Val::Px(sides),
            height: Val::Px(sides),
            border: UiRect::all(Val::Px(5.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(30.),
            ..default()
        },
        border_color: BorderColor(Color::BLACK),
        background_color: BackgroundColor(NORMAL_BUTTON),
        ..default()
    })
}

fn audio_menu() -> Column<NodeBundle> {
    menu_base(500.)
}

fn graphics_menu() -> Column<NodeBundle> {
    menu_base(500.)
}

fn x_button(mut on_press: impl FnMut() + 'static + Send + Sync) -> impl Element {
    let hovered = Mutable::new(false);
    El::from(
        ButtonBundle {
            background_color: BackgroundColor(Color::NONE),
            ..default()
        }
    )
    .on_hovered_change(clone!((hovered) move |is_hovered| hovered.set_neq(is_hovered)))
    .on_press(move |is_pressed| if is_pressed { on_press() })
    .child(
        El::<TextBundle>::new()
        .text_signal(
            hovered.signal()
            .map_bool(|| Color::RED, || TEXT_COLOR)
            .map(|color| {
                Text::from_section("x", TextStyle {
                    font_size: 30.0,
                    color,
                    ..default()
                })
            })
        )
    )
}

fn menu() -> impl Element {
    let show_sub_menu = Mutable::new(None);
    Stack::<NodeBundle>::new()
    .layer(
        menu_base(300.)
        .items([
            button(SubMenu::Audio, show_sub_menu.clone()),
            button(SubMenu::Graphics, show_sub_menu.clone()),
        ])
    )
    .layer_signal(
        show_sub_menu.signal()
        .map_some(
            move |sub_menu| {
                let menu = match sub_menu {
                    SubMenu::Audio => audio_menu(),
                    SubMenu::Graphics => graphics_menu(),
                };
                Stack::<NodeBundle>::new()
                .layer(
                    menu.update_raw_el(|raw_el| {
                        raw_el.update_component::<Style>(|style| {
                            style.position_type = PositionType::Absolute;
                            style.align_self = AlignSelf::Center;
                            style.justify_self = JustifySelf::Center;
                        })
                    })
                )
                .layer(x_button(clone!((show_sub_menu) move || { show_sub_menu.take(); })))
                .update_raw_el(|raw_el| {
                    raw_el.update_component::<Style>(|style| {
                        style.position_type = PositionType::Absolute;
                        style.align_self = AlignSelf::Center;
                        style.justify_self = JustifySelf::Center;
                    })
                })
            },
        )
    )
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn insert_ui_root(world: &mut World) {
    let root_node = El::from(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ..default()
    });
    root_node.child(menu()).spawn(world);
}
