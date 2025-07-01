#![allow(dead_code)]
//! Simple counter.

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

#[derive(Component)]
struct Counter(Mutable<i32>);

fn ui_root() -> impl Element {
    let counter = Mutable::new(0);
    El::<Node>::new()
        .with_node(|mut node| node.height = Val::Percent(100.))
        .with_node(|mut node| node.width = Val::Percent(100.))
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.0))
                .item(counter_button(counter.clone(), "-", -1))
                .item(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(25.))
                        .text_signal(counter.signal_ref(ToString::to_string).map(Text)),
                )
                .item(counter_button(counter.clone(), "+", 1))
                .update_raw_el(move |raw_el| raw_el.insert(Counter(counter))),
        )
}

fn counter_button(counter: Mutable<i32>, label: &str, step: i32) -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .with_node(|mut node| node.width = Val::Px(45.0))
        .align_content(Align::center())
        .border_radius(BorderRadius::MAX)
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .on_click(move || *counter.lock_mut() += step)
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(label)),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
