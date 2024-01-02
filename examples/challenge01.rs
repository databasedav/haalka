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
        .add_systems(Startup, (setup, spawn_ui_root))
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
    El::<ButtonBundle>::new()
    .with_style(|style| {
        style.width = Val::Px(180.0);
        style.height = Val::Px(65.);
        style.border = UiRect::all(Val::Px(5.));
    })
    .align_content(vec![Align::CenterX, Align::CenterY])
    .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    .on_pressed_change(move |is_pressed| {
        if is_pressed { show_sub_menu.set_neq(Some(sub_menu)) }
        pressed.set_neq(is_pressed);
    })
    .border_color_signal(border_color_signal)
    .background_color_signal(background_color_signal)
    .child(
        El::<TextBundle>::new()
        .text(
            Text::from_section(
                match sub_menu { SubMenu::Audio => "audio", SubMenu::Graphics => "graphics" },
                TextStyle { font_size: 40.0, color: TEXT_COLOR, ..default() }
            )
        )
    )
}

fn menu_base(sides: f32) -> Column<NodeBundle> {
    Column::<NodeBundle>::new()
    .with_style(move |style| {
        style.width = Val::Px(sides);
        style.height = Val::Px(sides);
        style.border = UiRect::all(Val::Px(5.));
        style.row_gap = Val::Px(30.);
    })
    .border_color(BorderColor(Color::BLACK))
    .background_color(BackgroundColor(NORMAL_BUTTON))
    .align_content(vec![Align::CenterX, Align::CenterY])
}

fn audio_menu() -> Column<NodeBundle> {
    menu_base(500.)
}

fn graphics_menu() -> Column<NodeBundle> {
    menu_base(500.)
}

fn x_button(mut on_press: impl FnMut() + 'static + Send + Sync) -> impl Element + RawElWrapper + Alignable {
    let hovered = Mutable::new(false);
    El::<ButtonBundle>::new()
    .background_color(BackgroundColor(Color::NONE))
    .on_hovered_change(clone!((hovered) move |is_hovered| hovered.set_neq(is_hovered)))
    .on_pressed_change(move |is_pressed| if is_pressed { on_press() })
    .child(
        El::<TextBundle>::new()
        .text(Text::from_section("x", TextStyle { font_size: 30.0, ..default() }))
        .signal_with_text(
            hovered.signal().map_bool(|| Color::RED, || TEXT_COLOR),
            |text, color| {
                if let Some(section) = text.sections.first_mut() {
                    section.style.color = color;
                }
            },
        )
        // or like this:
        // .text_signal(
        //     hovered.signal().map_bool(|| Color::RED, || TEXT_COLOR)
        //     .map(|color| {
        //         Text::from_section("x", TextStyle {
        //             font_size: 30.0,
        //             color,
        //             ..default()
        //         })
        //     })
        // )
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
                .with_style(|style| {
                    style.width =  Val::Px(500.);
                    style.height =  Val::Px(500.);
                    // TODO: without absolute there's some weird bouncing when switching between menus
                    style.position_type =  PositionType::Absolute;
                })
                .align(vec![Align::CenterX, Align::CenterY])
                .layer(menu.align(vec![Align::CenterX, Align::CenterY]))
                .layer(
                    x_button(clone!((show_sub_menu) move || { show_sub_menu.take(); }))
                    .align(vec![Align::Top, Align::Right])
                    .update_raw_el(|raw_el| {
                        raw_el.with_component::<Style>(|style| {
                            style.padding = UiRect::new(Val::Px(0.), Val::Px(10.), Val::Px(5.), Val::Px(0.));
                        })
                    })
                )
            }
        )
    )
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
    .child(menu())
    .spawn(world);
}
