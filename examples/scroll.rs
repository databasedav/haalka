//! Scrollable row of scrollable letter columns. Inspired by <https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs>.

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
        .add_systems(Update, shifter)
        .insert_resource(Shifted(false))
        .run();
}

const LETTER_SIZE: f32 = 54.167; // 65 / 1.2
const CELL_SIZE: f32 = 66.;
const NUM_VISIBLE_COLUMNS: usize = 5;

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct Shifted(bool);

fn letter(letter: String, color: Color) -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(CELL_SIZE);
            node.height = Val::Px(CELL_SIZE);
        })
        .align_content(Align::center())
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(LETTER_SIZE))
                .text_color(TextColor(color))
                .text(Text::new(letter)),
        )
}

fn letter_column(rotate: usize, color: Color) -> impl Element {
    let lazy_entity = LazyEntity::new();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let shifted = signal::from_resource_changed::<Shifted>().map_in(deref_copied);
    Column::<Node>::new()
        .lazy_entity(lazy_entity)
        .insert((Pickable::default(), Hoverable))
        .with_node(|mut node| {
            node.width = Val::Px(CELL_SIZE);
            node.height = Val::Px(5. * CELL_SIZE);
        })
        .mutable_viewport(Overflow::scroll_y())
        .on_scroll_disableable_signal(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(CELL_SIZE)
                .into_system(),
            signal::any!(hovered.not(), shifted.clone()),
        )
        .cursor_signal(
            shifted
                .map_bool_in(|| SystemCursorIcon::EwResize, || SystemCursorIcon::NsResize)
                .map_in(CursorIcon::System)
                .dedupe(),
        )
        .with_scroll_position(move |mut scroll_position| scroll_position.y = CELL_SIZE * rotate as f32)
        .items(
            "abcdefghijklmnopqrstuvwxyz"
                .chars()
                .map(move |c| letter(c.to_string(), color)),
        )
}

fn ui_root() -> impl Element {
    let lazy_entity = LazyEntity::new();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let shifted = signal::from_resource_changed::<Shifted>().map_in(deref_copied);
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .lazy_entity(lazy_entity)
                .insert((Pickable::default(), Hoverable))
                .with_node(|mut node| {
                    node.width = Val::Px(CELL_SIZE * NUM_VISIBLE_COLUMNS as f32);
                })
                .mutable_viewport(Overflow::scroll_x())
                .on_scroll_disableable_signal(
                    BasicScrollHandler::new()
                        .direction(ScrollDirection::Horizontal)
                        // TODO: special handler for auto discrete like rectray https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs
                        .pixels(CELL_SIZE)
                        .into_system(),
                    signal::all!(hovered, shifted.clone()).not(),
                )
                .cursor_signal(
                    shifted
                        .map_bool_in(|| SystemCursorIcon::EwResize, || SystemCursorIcon::NsResize)
                        .map_in(CursorIcon::System)
                        .dedupe(),
                )
                .items(
                    [
                        bevy::color::palettes::css::RED,
                        bevy::color::palettes::css::ORANGE,
                        bevy::color::palettes::css::YELLOW,
                        bevy::color::palettes::css::GREEN,
                        bevy::color::palettes::css::BLUE,
                        bevy::color::palettes::css::INDIGO,
                        bevy::color::palettes::css::VIOLET,
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(i, color)| letter_column(i, color.into())),
                ),
        )
}

fn shifter(keys: Res<ButtonInput<KeyCode>>, mut shifted: ResMut<Shifted>) {
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) {
        **shifted = true;
    } else if keys.just_released(KeyCode::ShiftLeft) || keys.just_released(KeyCode::ShiftRight) {
        **shifted = false;
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
