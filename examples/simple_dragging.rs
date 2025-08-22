#![allow(dead_code)]
//! Simple dragging

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

static DRAGGING: LazyLock<Mutable<Option<Entity>>> = LazyLock::new(default);
static POINTER_POSITION: LazyLock<Mutable<Vec2>> = LazyLock::new(default);

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
        .background_color(BackgroundColor(Color::WHITE))
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

            node.top = Val::Px(100.0);
            node.left = Val::Px(100.0 * (i + 1) as f32)
        })
        .background_color(BackgroundColor(Color::srgb_u8(8, 18, 44)))
        .global_z_index(GlobalZIndex(1))
        .update_raw_el(|raw_el| {
            // Maybe use `observe` here to get the actual entity in case of bubbling?
            raw_el
                .insert(Pickable::default())
                .on_event::<Pointer<Pressed>>(|click| DRAGGING.set(Some(click.target)))
                .on_event::<Pointer<Released>>(|_| DRAGGING.set(None))
                .on_signal_with_entity(POINTER_POSITION.signal(), |mut entity, pos| {
                    let this_entity_id = entity.id();
                    if let Some((dragging_entity, node)) = DRAGGING.get().zip(entity.get_mut::<Node>())
                        && dragging_entity == this_entity_id
                    {
                        set_dragging_position(node, pos);
                    }
                })
        })
        .child(El::<Text>::new().text(Text::new(format!("{i}"))))
}

fn set_dragging_position(mut node: Mut<Node>, pointer_position: Vec2) {
    node.left = Val::Px(pointer_position.x - WIDTH / 2.);
    node.top = Val::Px(pointer_position.y - HEIGHT / 2.);
}
