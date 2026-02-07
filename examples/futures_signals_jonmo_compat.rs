//! Demonstrates that both futures-signals and jonmo signals backends can be used together.

mod utils;
use utils::*;

use bevy::{prelude::*, ui::Pressed};

#[allow(deprecated)]
use haalka::{
    futures_signals::prelude::*,
    prelude::{
        Align as jAlign, Alignable as _, BuilderPassThrough, BuilderWrapper, Draggable, El as jEl,
        Element as jElement, Hoverable, Hovered, LazyEntity, Pressable, SignalExt as _, signal,
    },
};

fn main() {
    #[allow(deprecated)]
    App::new()
        .add_plugins(examples_plugin)
        .add_plugins(haalka::futures_signals::HaalkaFuturesSignalsPlugin)
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

#[allow(deprecated)]
fn futures_signals_button() -> impl Element {
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    let pressed_hovered = map_ref!(pressed_signal, hovered_signal => (*pressed_signal, *hovered_signal)).broadcast();

    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(150.0);
            node.height = Val::Px(65.);
            node.border = UiRect::all(Val::Px(5.0));
        })
        .align_content(Align::center())
        .border_color_signal(
            pressed_hovered
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
                .map(BorderColor::all),
        )
        .background_color_signal(
            pressed_hovered
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
                .map(BackgroundColor),
        )
        .border_radius(BorderRadius::MAX)
        .hovered_sync(hovered)
        .pressed_sync(pressed)
        .child(
            El::<Text>::new()
                .text_font(TextFont {
                    font_size: 25.0,
                    ..default()
                })
                .text_shadow(TextShadow::default())
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text_signal(
                    pressed_hovered
                        .signal()
                        .map(|(pressed, hovered)| {
                            if pressed {
                                "Press"
                            } else if hovered {
                                "Hover"
                            } else {
                                "futures-signals"
                            }
                        })
                        .map(Text::new),
                ),
        )
}

fn jonmo_button() -> impl jElement {
    let lazy_entity = LazyEntity::new();

    let pressed = signal::from_entity(lazy_entity.clone())
        .has_component::<Pressed>()
        .dedupe();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let pressed_hovered = signal::zip!(pressed, hovered).dedupe();

    jEl::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(150.0);
            node.height = Val::Px(65.);
            node.border = UiRect::all(Val::Px(5.0));
        })
        .insert((Pickable::default(), Hoverable, Pressable, Draggable))
        .align_content(jAlign::center())
        .border_radius(BorderRadius::MAX)
        .lazy_entity(lazy_entity)
        .border_color_signal(
            pressed_hovered
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
            pressed_hovered
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
            jEl::<Text>::new()
                .text_font(TextFont {
                    font_size: 33.0,
                    ..default()
                })
                .text_shadow(TextShadow::default())
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text_signal(
                    pressed_hovered
                        .map_in(|(pressed, hovered)| {
                            if pressed {
                                "Press"
                            } else if hovered {
                                "Hover"
                            } else {
                                "jonmo"
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

#[allow(deprecated)]
fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| {
                    node.column_gap = Val::Px(50.);
                })
                .item(futures_signals_button())
                .item(El::<Node>::new().update_raw_el(|raw_el| {
                    raw_el.on_spawn(|world, entity| {
                        jonmo_button().into_builder().spawn_on_entity(world, entity).unwrap();
                    })
                })),
        )
}
