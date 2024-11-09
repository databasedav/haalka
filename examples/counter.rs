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
    El::<NodeBundle>::new()
        .height(Val::Percent(100.))
        .width(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<NodeBundle>::new()
                .with_style(|mut style| style.column_gap = Val::Px(15.0))
                .item(counter_button(counter.clone(), "-", -1))
                .item(El::<TextBundle>::new().text_signal(counter.signal().map(text)))
                .item(counter_button(counter.clone(), "+", 1))
                .update_raw_el(move |raw_el| raw_el.insert(Counter(counter))),
        )
}

fn counter_button(counter: Mutable<i32>, label: &str, step: i32) -> impl Element {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .on_click(move || *counter.lock_mut() += step)
        .child(El::<TextBundle>::new().text(text(label)))
}

fn text(text: impl ToString) -> Text {
    Text::from_section(
        text.to_string(),
        TextStyle {
            font_size: 30.0,
            ..default()
        },
    )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
