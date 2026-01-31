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

#[derive(Component, Clone, Deref, DerefMut)]
struct Counter(i32);

#[rustfmt::skip]
fn ui_root() -> impl Element {
    let counter_holder = LazyEntity::new();
    El::<Node>::new()
        .with_node(|mut node| {
            node.height = Val::Percent(100.);
            node.width = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.0))
                .insert(Counter(0))
                .lazy_entity(counter_holder.clone())
                .item(counter_button(counter_holder.clone(), "-", -1))
                .item(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(25.))
                        .text_signal(
                            signal::from_component_changed::<Counter>(counter_holder.clone())
                                .map_in(deref_copied)
                                .map_in_ref(ToString::to_string)
                                .map_in(Text)
                                .map_in(Some),
                        ),
                )
                .item(counter_button(counter_holder.clone(), "+", 1)),
        )
}

fn counter_button(counter_holder: LazyEntity, label: &'static str, step: i32) -> impl Element {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .with_node(|mut node| node.width = Val::Px(45.0))
        .insert((Pickable::default(), Hoverable))
        .align_content(Align::center())
        .border_radius(BorderRadius::MAX)
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            signal::from_entity(lazy_entity)
                .has_component::<Hovered>()
                .dedupe()
                .map_bool_in(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .on_click(move |_: In<_>, mut counters: Query<&mut Counter>| {
            if let Ok(mut counter) = counters.get_mut(*counter_holder) {
                **counter += step;
            }
        })
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(label)),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
