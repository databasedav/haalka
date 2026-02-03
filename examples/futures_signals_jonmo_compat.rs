//! Demonstrates that both futures-signals and jonmo backends can be used together.
//!
//! This example shows two buttons side by side:
//! - Left button uses the deprecated futures-signals backend (existing code)
//! - Right button uses the new jonmo-based backend (code being migrated to)
//!
//! The UI root uses futures-signals, showing how users can incrementally migrate
//! from futures-signals to jonmo while keeping both working in tandem.
//!
//! Run with: `cargo run --example futures_signals_jonmo_compat --features futures_signals_ui`

mod utils;
use utils::*;

use bevy::{prelude::*, ui::Pressed};

// ============================================================================
// FUTURES-SIGNALS IMPORTS (the existing/deprecated approach)
// ============================================================================
#[allow(deprecated)]
use haalka::futures_signals::prelude::*;

// ============================================================================
// JONMO IMPORTS (the new recommended approach)
// These are imported with aliases to avoid conflicts with futures-signals types.
// ============================================================================
mod jonmo_compat {
    pub use haalka::{
        align::Alignable as JonmoAlignable,
        element::{BuilderPassThrough as JonmoBPT, BuilderWrapper as JonmoBW},
        jonmo::signal::{self as signal, SignalExt as JonmoSignalExt},
        pointer_event_aware::{Draggable, Dragged, Hoverable, Hovered, Pressable},
        prelude::{Align as JonmoAlign, El as JonmoEl, LazyEntity},
    };
}

fn main() {
    #[allow(deprecated)]
    App::new()
        .add_plugins(examples_plugin)
        // Add the futures-signals plugin for the deprecated backend
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

// ============================================================================
// FUTURES-SIGNALS BASED BUTTON (existing code users are migrating FROM)
// This is essentially unchanged from the original button.rs on git origin main.
// ============================================================================

#[allow(deprecated)]
fn futures_signals_button() -> impl Element {
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    let pressed_hovered_broadcaster =
        map_ref!(pressed_signal, hovered_signal => (*pressed_signal, *hovered_signal)).broadcast();

    let border_color_signal = pressed_hovered_broadcaster
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
        .map(BorderColor::all);

    let background_color_signal = pressed_hovered_broadcaster
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
        .map(BackgroundColor);

    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(150.0);
            node.height = Val::Px(65.);
            node.border = UiRect::all(Val::Px(5.0));
        })
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .align_content(Align::center())
        .border_color_signal(border_color_signal)
        .background_color_signal(background_color_signal)
        .border_radius(BorderRadius::MAX)
        .hovered_sync(hovered)
        .pressed_sync(pressed)
        .child(
            El::<Text>::new()
                .text_font(TextFont {
                    font_size: 25.0,
                    ..default()
                })
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text_signal(
                    pressed_hovered_broadcaster
                        .signal()
                        .map(|(pressed, hovered)| {
                            if pressed {
                                "FS Press"
                            } else if hovered {
                                "FS Hover"
                            } else {
                                "FuturesSignals"
                            }
                        })
                        .map(Text::new),
                ),
        )
}

// ============================================================================
// JONMO-BASED BUTTON (new code users are migrating TO)
// ============================================================================

/// Creates a jonmo-based button element.
/// This demonstrates how to build new UI components using the jonmo backend
/// while the rest of the app still uses futures-signals.
fn jonmo_button() -> jonmo_compat::JonmoEl<Node> {
    use jonmo_compat::{
        Draggable, Dragged, Hoverable, Hovered, JonmoAlign, JonmoAlignable as _, JonmoBPT as _, JonmoEl,
        JonmoSignalExt as _, LazyEntity, Pressable, signal as jonmo_signal,
    };

    let lazy_entity = LazyEntity::new();

    let pressed = jonmo_signal::from_entity(lazy_entity.clone())
        .has_component::<Pressed>()
        .dedupe();
    let dragged = jonmo_signal::from_entity(lazy_entity.clone())
        .has_component::<Dragged>()
        .dedupe();
    let hovered = jonmo_signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let pressed_hovered = jonmo_signal::zip!(jonmo_signal::any!(pressed, dragged), hovered).dedupe();

    let border_color_signal = pressed_hovered
        .clone()
        .map_in(|(pressed, hovered)| {
            if pressed {
                bevy::color::palettes::basic::BLUE.into()
            } else if hovered {
                Color::WHITE
            } else {
                Color::BLACK
            }
        })
        .map_in(BorderColor::all)
        .map_in(Some);

    let background_color_signal = pressed_hovered
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
        .map_in(Some);

    let text_signal = pressed_hovered
        .map_in(|(pressed, hovered)| {
            if pressed {
                "Jonmo Press"
            } else if hovered {
                "Jonmo Hover"
            } else {
                "Jonmo"
            }
        })
        .map_in(Text::new)
        .map_in(Some);

    JonmoEl::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(150.0);
            node.height = Val::Px(65.);
            node.border = UiRect::all(Val::Px(5.0));
        })
        .insert((Pickable::default(), Hoverable, Pressable, Draggable, BorderRadius::MAX))
        .align_content(JonmoAlign::center())
        .lazy_entity(lazy_entity)
        .border_color_signal(border_color_signal)
        .background_color_signal(background_color_signal)
        .child(
            JonmoEl::<Text>::new()
                .text_font(TextFont {
                    font_size: 25.0,
                    ..default()
                })
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text_signal(text_signal),
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
                .item(
                    // futures-signals button with label
                    Column::<Node>::new()
                        .with_node(|mut node| {
                            node.row_gap = Val::Px(10.);
                        })
                        .align(Align::center())
                        .item(El::<Text>::new().text(Text::new("futures-signals\n(deprecated)")))
                        .item(futures_signals_button()),
                )
                .item(
                    // jonmo button with label - injected via on_spawn hook
                    Column::<Node>::new()
                        .with_node(|mut node| {
                            node.row_gap = Val::Px(10.);
                        })
                        .align(Align::center())
                        .item(El::<Text>::new().text(Text::new("jonmo\n(recommended)")))
                        .item(El::<Node>::new().update_raw_el(|raw_el| {
                            raw_el.on_spawn(|world, entity| {
                                use jonmo_compat::JonmoBW as _;
                                let _ = jonmo_button().into_builder().spawn_on_entity(world, entity);
                            })
                        })),
                ),
        )
}
