use bevy::prelude::*;
use haalka::*;
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals_ext::*;

fn main() {
    App::new()
    .add_plugins((DefaultPlugins, HaalkaPlugin))
    .add_systems(Startup, (ui_root, camera))
    .run();
}

#[derive(Component)]
struct Counter(Mutable<i32>);

fn ui_root(world: &mut World) {
    let counter = Mutable::new(0);
    El::<NodeBundle>::new()
    .with_style(|style| {
        style.width = Val::Percent(100.);
        style.height = Val::Percent(100.);
    })
    .align_content(Align::center())
    .child(
        Row::<NodeBundle>::new()
        .with_style(|style| style.column_gap = Val::Px(15.0))
        .item(counter_button(counter.clone(), "-", -1))
        .item(
            El::<TextBundle>::new()
            .text_signal(
                counter.signal()
                .map(|count| {
                    Text::from_section(
                        count.to_string(),
                        TextStyle { font_size: 30.0, ..default() },
                    )
                })
            )
        )
        .item(counter_button(counter.clone(), "+", 1))
        .update_raw_el(move |raw_el| raw_el.insert(Counter(counter)))
    )
    .spawn(world);
}

fn counter_button(counter: Mutable<i32>, label: &str, step: i32) -> impl Element {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
    .with_style(|style| style.width = Val::Px(45.0))
    .align_content(Align::center())
    .background_color_signal(
        hovered.signal()
        .map_bool(
            || Color::hsl(300., 0.75, 0.85),
            || Color::hsl(300., 0.75, 0.75),
        )
        .map(BackgroundColor)
    )
    .hovered_sync(hovered)
    .on_click(move || *counter.lock_mut() += step)
    .child(
        El::<TextBundle>::new()
        .text(Text::from_section(label, TextStyle { font_size: 30.0, ..default() }))
    )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}