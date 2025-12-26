//! Experimental port of <https://github.com/aevyrie/bevy_mod_picking/blob/main/examples/many_buttons.rs>.

mod utils;
use bevy_ui::Pressed;
use jonmo::builder::JonmoBuilder;
use utils::*;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((examples_plugin, LogDiagnosticsPlugin::default()))
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

const SIZE: usize = 20; // SIZE^2 buttons
const FONT_SIZE: f32 = 5.83;
const HOVERED_COLOR: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_COLOR: Color = Color::srgb(0.35, 0.75, 0.35);

fn button(i: usize, j: usize) -> JonmoBuilder {
    let color = as_rainbow(j % i.max(1));
    let lazy_entity = LazyEntity::new();

    let pressed_hovered_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
        // Can't use .has_component because Pressed is not Clone
        .map(|In(entity), presseds: Query<&Pressed>| presseds.contains(entity))
        .dedupe()
        .combine(
            SignalBuilder::from_lazy_entity(lazy_entity.clone())
                .has_component::<Hovered>()
                .dedupe(),
        )
        .dedupe();

    let background_color_signal = pressed_hovered_signal
        .map_in(move |(pressed, hovered)| {
            if pressed {
                PRESSED_COLOR
            } else if hovered {
                HOVERED_COLOR
            } else {
                color
            }
        })
        .map_in(BackgroundColor)
        .map_in(Some);

    let total = SIZE as f32;
    let width = 90. / total;
    JonmoBuilder::new()
        .insert(Node::default())
        .insert(Pickable::default())
        .with_component::<Node>(move |mut node| {
            node.width = Val::Percent(width);
            node.height = Val::Percent(width);
            node.bottom = Val::Percent(100. / total * i as f32);
            node.left = Val::Percent(100. / total * j as f32);
            node.align_items = AlignItems::Center;
            node.position_type = PositionType::Absolute;
            node.border = UiRect::all(Val::Percent(10. / total));
        })
        .lazy_entity(lazy_entity)
        .component_signal::<BackgroundColor, _>(background_color_signal)
        .insert(BorderColor::all(as_rainbow(i % j.max(1))))
        .child(JonmoBuilder::from((
            Text(format!("{i} {j}")),
            TextFont::from_font_size(FONT_SIZE),
            TextColor(Color::srgb(0.2, 0.2, 0.2)),
        )))
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn as_rainbow(i: usize) -> Color {
    Color::hsl((i as f32 / SIZE as f32) * 360.0, 0.9, 0.8)
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.flex_direction = FlexDirection::Column;
            node.justify_content = JustifyContent::Center;
            node.align_items = AlignItems::Center;
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .with_builder(|builder| builder.children((0..SIZE).flat_map(|i| (0..SIZE).map(move |j| button(i, j)))))
}
