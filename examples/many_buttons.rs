//! Experimental port of https://github.com/aevyrie/bevy_mod_picking/blob/main/examples/many_buttons.rs.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            HaalkaPlugin,
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, (setup, spawn_ui_root))
        .run();
}

const SIZE: usize = 110; // SIZE^2 buttons
const FONT_SIZE: f32 = 5.83;
const HOVERED_COLOR: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_COLOR: Color = Color::srgb(0.35, 0.75, 0.35);

fn button(i: usize, j: usize) -> RawHaalkaEl {
    let color = as_rainbow(j % i.max(1));
    let (_pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (_hovered, hovered_signal) = Mutable::new_and_signal(false);
    let background_color_signal = {
        map_ref!(pressed_signal, hovered_signal => {
            if *pressed_signal {
                PRESSED_COLOR
            } else if *hovered_signal {
                HOVERED_COLOR
            } else {
                color
            }
        })
        .map(BackgroundColor)
    };
    let total = SIZE as f32;
    let width = 90. / total;
    RawHaalkaEl::new()
        .insert(Node::default())
        .with_component::<Node>(move |mut node| {
            node.width = Val::Percent(width);
            node.height = Val::Percent(width);
            node.bottom = Val::Percent(100. / total * i as f32);
            node.left = Val::Percent(100. / total * j as f32);
            node.align_items = AlignItems::Center;
            node.position_type = PositionType::Absolute;
            node.border = UiRect::all(Val::Percent(10. / total));
        })
        .component_signal(background_color_signal)
        .insert(BorderColor(as_rainbow(i % j.max(1))))
        // .hovered_sync(hovered)
        // .pressed_sync(pressed)
        .child(RawHaalkaEl::new().insert((
            Text(format!("{i} {j}")),
            TextFont::from_font_size(FONT_SIZE),
            TextColor(Color::srgb(0.2, 0.2, 0.2)),
        )))
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn as_rainbow(i: usize) -> Color {
    Color::hsl((i as f32 / SIZE as f32) * 360.0, 0.9, 0.8)
}

fn spawn_ui_root(world: &mut World) {
    RawHaalkaEl::new()
        .insert(Node::default())
        .with_component::<Node>(|mut node| {
            node.flex_direction = FlexDirection::Column;
            node.justify_content = JustifyContent::Center;
            node.align_items = AlignItems::Center;
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .children((0..SIZE).flat_map(|i| (0..SIZE).map(move |j| button(i, j))))
        .spawn(world);
}
