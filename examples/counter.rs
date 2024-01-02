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
    El::from(
        NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            ..default()
        }
    )
    .align_content(vec![Align::CenterX, Align::CenterY])
    .child(
        Row::from(
            NodeBundle {
                style: Style { column_gap: Val::Px(15.0), ..default() },
                ..default()
            }
        )
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
    let pressed = Mutable::new(false);
    El::from(ButtonBundle {
        style: Style {
            width: Val::Px(45.0),
            ..default()
        },
        ..default()
    })
    .align_content(vec![Align::CenterX, Align::CenterY])
    .background_color_signal(
        signal::or(hovered.signal(), pressed.signal()).dedupe()
        .map_bool(
            || Color::hsl(300., 0.75, 0.85),
            || Color::hsl(300., 0.75, 0.75),
        )
        .map(BackgroundColor)
    )
    .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    .on_pressed_change(move |is_pressed| {
        if is_pressed { *counter.lock_mut() += step }
        pressed.set_neq(is_pressed);
    })
    .child(El::from(TextBundle::from_section(label, TextStyle { font_size: 30.0, ..default() })))
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
