//! Dragging with z-index stacking

mod utils;
use utils::*;

use bevy::{prelude::*, ui::Pressed};
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
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        })
        .insert(Pickable::default())
        .cursor_disableable_signal(
            CursorIcon::System(SystemCursorIcon::Default),
            signal::any!(
                SignalBuilder::from_system(|In(_), presseds: Query<&Pressed>| !presseds.is_empty()).dedupe(),
                SignalBuilder::from_system(|In(_), draggeds: Query<&Dragged>| !draggeds.is_empty()).dedupe(),
            ),
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
        .background_color(BackgroundColor(random_color()))
        .global_z_index(GlobalZIndex(1))
        .cursor_signal(
            signal::any!(
                SignalBuilder::from_lazy_entity(lazy_entity.clone())
                    .has_component::<Pressed>()
                    .dedupe(),
                SignalBuilder::from_lazy_entity(lazy_entity.clone())
                    .has_component::<Dragged>()
                    .dedupe(),
            )
            .dedupe()
            .combine(
                SignalBuilder::from_lazy_entity(lazy_entity.clone())
                    .has_component::<Hovered>()
                    .dedupe(),
            )
            .map_in(|(dragged, hovered)| match (dragged, hovered) {
                (true, _) => SystemCursorIcon::Grabbing,
                (false, true) => SystemCursorIcon::Grab,
                (false, false) => SystemCursorIcon::Default,
            })
            .map_in(CursorIcon::System)
            .dedupe(),
        )
        .lazy_entity(lazy_entity.clone())
        .insert((Pickable::default(), DragOffset::default()))
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
        .on_dragging(
            |In((entity, dragging_data)): In<(Entity, DraggingData)>, mut nodes: Query<(&mut Node, &DragOffset)>| {
                if let Ok((node, drag_offset)) = nodes.get_mut(entity) {
                    set_dragging_position(node, dragging_data.pointer_location.position, drag_offset.0);
                }
            },
        )
        .child(El::<Text>::new().text(Text::new(format!("{i}"))))
}

fn set_dragging_position(mut node: Mut<Node>, pointer_position: Vec2, offset: Vec2) {
    node.left = Val::Px(pointer_position.x - offset.x);
    node.top = Val::Px(pointer_position.y - offset.y);
}
