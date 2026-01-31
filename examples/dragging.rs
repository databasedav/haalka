//! Dragging with z-index stacking

mod utils;
use utils::*;

use bevy::{prelude::*, ui::Pressed};
use bevy_rand::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<WyRand>::default()))
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
                },
                camera,
            ),
        )
        .insert_resource(MaxZIndex::default())
        .run();
}

#[derive(Resource, Default)]
struct MaxZIndex(i32);

#[derive(Component, Default)]
struct DragOffset(Vec2);

const WIDTH: f32 = 100.0;
const HEIGHT: f32 = 100.0;

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn calculate_and_set_drag_offset(
    entity: Entity,
    press_position: Vec2,
    nodes: &Query<&Node>,
    mut drag_offsets: Query<&mut DragOffset>,
) {
    if let Ok(node) = nodes.get(entity) {
        let left = match node.left {
            Val::Px(px) => px,
            _ => 0.0,
        };
        let top = match node.top {
            Val::Px(px) => px,
            _ => 0.0,
        };
        let offset = Vec2::new(press_position.x - left, press_position.y - top);
        if let Ok(mut drag_offset) = drag_offsets.get_mut(entity) {
            drag_offset.0 = offset;
        }
    }
}

fn update_z_index(entity: Entity, mut max_z_index: ResMut<MaxZIndex>, mut z_indices: Query<&mut GlobalZIndex>) {
    max_z_index.0 += 1;
    if let Ok(mut z_index) = z_indices.get_mut(entity) {
        z_index.0 = max_z_index.0;
    }
}

fn ui_root() -> impl Element {
    let any_pressed = signal::from_system(|In(_), presseds: Query<&Pressed>| !presseds.is_empty()).dedupe();
    let any_dragged = signal::from_system(|In(_), draggeds: Query<&Dragged>| !draggeds.is_empty()).dedupe();
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        })
        .insert(Pickable::default())
        .cursor_disableable_signal(
            CursorIcon::System(SystemCursorIcon::Default),
            signal::any!(any_pressed, any_dragged).dedupe(),
        )
        .align_content(Align::center())
        .child(Row::<Node>::new().items((0..=5).map(square)))
}

fn square(i: usize) -> impl Element {
    let lazy_entity = LazyEntity::new();

    El::<Node>::new()
        .with_node(move |mut node| {
            node.width = Val::Px(WIDTH);
            node.height = Val::Px(HEIGHT);
        })
        .align_content(Align::center())
        .with_builder(|builder| {
            builder.on_spawn_with_system(
                |In(entity): In<Entity>,
                 mut rng: Single<&mut WyRand, With<GlobalRng>>,
                 mut backgrounds: Query<&mut BackgroundColor>| {
                    if let Ok(mut bg) = backgrounds.get_mut(entity) {
                        *bg = BackgroundColor(random_color(rng.as_mut()));
                    }
                },
            )
        })
        .global_z_index(GlobalZIndex(1))
        .cursor_signal({
            let pressed = signal::from_entity(lazy_entity.clone())
                .has_component::<Pressed>()
                .dedupe();
            let dragged = signal::from_entity(lazy_entity.clone())
                .has_component::<Dragged>()
                .dedupe();
            let hovered = signal::from_entity(lazy_entity.clone())
                .has_component::<Hovered>()
                .dedupe();
            signal::zip!(signal::any!(pressed, dragged).dedupe(), hovered)
                .dedupe()
                .map_in(|(dragged, hovered)| {
                    if dragged {
                        SystemCursorIcon::Grabbing
                    } else if hovered {
                        SystemCursorIcon::Grab
                    } else {
                        SystemCursorIcon::Default
                    }
                })
                .map_in(CursorIcon::System)
                .dedupe()
        })
        .lazy_entity(lazy_entity.clone())
        .insert((Pickable::default(), Hoverable, Pressable, DragOffset::default()))
        .observe(
            |click: On<Pointer<Press>>,
             max_z_index: ResMut<MaxZIndex>,
             z_indices: Query<&mut GlobalZIndex>,
             drag_offsets: Query<&mut DragOffset>,
             nodes: Query<&Node>| {
                calculate_and_set_drag_offset(click.entity, click.pointer_location.position, &nodes, drag_offsets);
                update_z_index(click.entity, max_z_index, z_indices);
            },
        )
        .on_dragged(
            |In((entity, drag_data)): In<(Entity, DragData)>, mut nodes: Query<(&mut Node, &DragOffset)>| {
                if drag_data.dragged {
                    if let Ok((node, drag_offset)) = nodes.get_mut(entity) {
                        set_dragging_position(node, drag_data.pointer_location.position, drag_offset.0);
                    }
                }
            },
        )
        .child(El::<Text>::new().text(Text::new(i.to_string())))
}

fn set_dragging_position(mut node: Mut<Node>, pointer_position: Vec2, offset: Vec2) {
    node.left = Val::Px(pointer_position.x - offset.x);
    node.top = Val::Px(pointer_position.y - offset.y);
}
