//! Dragging with z-index stacking

mod utils;
use utils::*;

use bevy::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .init_resource::<MaxZIndex>()
        .init_resource::<Dragging>()
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

static POINTER_POSITION: LazyLock<Mutable<Vec2>> = LazyLock::new(default);

#[derive(Resource, Default)]
struct Dragging(Option<Entity>);

#[derive(Resource, Default)]
struct MaxZIndex(i32);

#[derive(Component, Default)]
struct DragOffset(Vec2);

const WIDTH: f32 = 100.0;
const HEIGHT: f32 = 100.0;

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        })
        .update_raw_el(|raw_el| {
            raw_el
                .on_event_with_system::<Pointer<Move>, _>(|In((_, move_)): In<(_, Pointer<Move>)>| {
                    POINTER_POSITION.set(move_.pointer_location.position);
                })
                .insert(Pickable::default())
        })
        .child(Row::<Node>::new().items((0..=5).map(square)))
}

fn square(i: usize) -> impl Element {
    El::<Node>::new()
        .with_node(move |mut node| {
            node.position_type = PositionType::Absolute;
            node.width = Val::Px(WIDTH);
            node.height = Val::Px(HEIGHT);
            node.justify_content = JustifyContent::Center;
            node.align_items = AlignItems::Center;

            node.top = Val::Px(100.0);
            node.left = Val::Px(100.0 * (i + 1) as f32)
        })
        .background_color(BackgroundColor(random_color()))
        .global_z_index(GlobalZIndex(1))
        .update_raw_el(|raw_el| {
            // Maybe use `observe` here to get the actual entity in case of bubbling?
            raw_el
                .insert((Pickable::default(), DragOffset::default()))
                .on_event_with_system::<Pointer<Pressed>, _>(
                    |In((_, click)): In<(_, Pointer<Pressed>)>,
                     mut dragging: ResMut<Dragging>,
                     mut max_z_index: ResMut<MaxZIndex>,
                     mut z_indices: Query<&mut GlobalZIndex>,
                     mut drag_offsets: Query<&mut DragOffset>,
                     nodes: Query<&Node>| {
                        let node = nodes.get(click.target).unwrap();
                        let left = match node.left {
                            Val::Px(px) => px,
                            _ => 0.0,
                        };
                        let top = match node.top {
                            Val::Px(px) => px,
                            _ => 0.0,
                        };
                        let offset = Vec2::new(
                            click.pointer_location.position.x - left,
                            click.pointer_location.position.y - top,
                        );
                        drag_offsets.get_mut(click.target).unwrap().0 = offset;
                        dragging.0 = Some(click.target);
                        max_z_index.0 += 1;
                        if let Ok(mut z_index) = z_indices.get_mut(click.target) {
                            z_index.0 = max_z_index.0;
                        }
                    },
                )
                .on_event_with_system::<Pointer<Released>, _>(
                    |In(_): In<_>, mut dragging: ResMut<Dragging>| {
                        dragging.0 = None;
                    },
                )
                .on_signal_with_entity(POINTER_POSITION.signal(), |mut entity, pos| {
                    let this_entity_id = entity.id();
                    if let Some(dragging_entity) = entity.world().resource::<Dragging>().0
                        && dragging_entity == this_entity_id
                    {
                        let offset = entity
                            .get::<DragOffset>()
                            .map(|d| d.0)
                            .unwrap_or(Vec2::new(WIDTH / 2., HEIGHT / 2.));
                        if let Some(node) = entity.get_mut::<Node>() {
                            set_dragging_position(node, pos, offset);
                        }
                    }
                })
        })
        .child(El::<Text>::new().text(Text::new(format!("{i}"))))
}

fn set_dragging_position(mut node: Mut<Node>, pointer_position: Vec2, offset: Vec2) {
    node.left = Val::Px(pointer_position.x - offset.x);
    node.top = Val::Px(pointer_position.y - offset.y);
}
