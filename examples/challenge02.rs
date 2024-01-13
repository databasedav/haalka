// Fixed-size grid, some spaces with items and some empty.
// Each item slot has an image of the item and the item count overlayed on the image.
// Items can be moved with drag and drop.
//     Both image and item count move along with the cursor while dragging.
//     The image and item count are not visible in the original position while dragging.
//     You can leave the bounding box of the inventory while dragging.
// A tooltip with the item's name is shown when hovering over an item.

use bevy::prelude::*;
use futures_signals::{
    map_ref,
    signal::{Mutable, SignalExt},
};
use haalka::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin,
        ))
        .add_systems(Startup, (setup, spawn_ui_root))
        .run();
}

const CELL_WIDTH: f32 = 70.;
const INVENTORY_BACKGROUND_COLOR: Color = Color::hsl(0., 0., 0.78);
const CELL_BACKGROUND_COLOR: Color = Color::hsl(0., 0., 0.55);
const CELL_GAP: f32 = 5.;
const INVENTORY_SIZE: f32 = 700.;
const CELL_BORDER_WIDTH: f32 = 2.;
const CELL_DARK_BORDER_COLOR: Color = Color::hsl(0., 0., 0.19);
const CELL_LIGHT_BORDER_COLOR: Color = Color::hsl(0., 0., 0.98);

fn cell() -> impl Element + Alignable {
    El::<NodeBundle>::new().child(
        El::<NodeBundle>::new()
            .with_style(|style| {
                style.width = Val::Px(CELL_WIDTH);
                style.height = Val::Px(CELL_WIDTH);
                style.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
            })
            .background_color(CELL_BACKGROUND_COLOR.into())
            .border_color(CELL_DARK_BORDER_COLOR.into()),
    )
}

fn grid(n: usize) -> Grid<NodeBundle> {
    Grid::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
            style.column_gap = Val::Px(CELL_GAP);
            style.row_gap = Val::Px(CELL_GAP);
        })
        .row_wrap_cell_width(CELL_WIDTH)
        .cells((0..n).into_iter().map(|_| cell()))
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn dot() -> impl Element {
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Px(CELL_BORDER_WIDTH * 2.);
            style.height = Val::Px(CELL_BORDER_WIDTH * 2.);
        })
        .background_color(CELL_BACKGROUND_COLOR.into())
}

fn dot_row(n: usize) -> impl Element {
    Row::<NodeBundle>::new().items((0..n).into_iter().map(|_| dot()))
}

fn arrow() -> impl Element {
    Column::<NodeBundle>::new()
        .align_content(Align::center())
        .items((0..=6).into_iter().map(|i| dot_row(2 * i + 1)))
        .item(
            El::<NodeBundle>::new()
                .with_style(|style| {
                    style.width = Val::Px(CELL_BORDER_WIDTH * 2. * 3.);
                    style.height = Val::Px(CELL_BORDER_WIDTH * 2. * 3. * 2.);
                })
                .background_color(CELL_BACKGROUND_COLOR.into()),
        )
}

fn inventory() -> impl Element {
    El::<NodeBundle>::new()
        .align_content(Align::center())
        .with_style(|style| {
            style.height = Val::Px(INVENTORY_SIZE);
            style.width = Val::Px(INVENTORY_SIZE);
        })
        .child(
            Column::<NodeBundle>::new()
                .with_style(|style| {
                    style.height = Val::Percent(100.);
                    style.width = Val::Percent(100.);
                    style.row_gap = Val::Px(CELL_GAP * 4.);
                })
                .background_color(INVENTORY_BACKGROUND_COLOR.into())
                .align_content(Align::center())
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|style| {
                            style.column_gap = Val::Px(CELL_GAP);
                            style.width = Val::Percent(100.);
                        })
                        .item(
                            Row::<NodeBundle>::new()
                                .align_content(Align::center())
                                .with_style(|style| {
                                    style.column_gap = Val::Px(CELL_GAP);
                                    style.width = Val::Percent(60.);
                                    style.padding = UiRect::horizontal(Val::Px(CELL_GAP * 3.));
                                })
                                .item(
                                    Column::<NodeBundle>::new()
                                        .with_style(|style| style.row_gap = Val::Px(CELL_GAP))
                                        .items((0..4).into_iter().map(|_| cell())),
                                )
                                .item(
                                    El::<NodeBundle>::new()
                                        .with_style(|style| {
                                            style.height = Val::Px(CELL_WIDTH * 4. + CELL_GAP * 3.);
                                            style.width = Val::Percent(100.);
                                        })
                                        .background_color(Color::BLACK.into()),
                                )
                                .item(
                                    Column::<NodeBundle>::new()
                                        .with_style(|style| style.row_gap = Val::Px(CELL_GAP))
                                        .items((0..4).into_iter().map(|_| cell())),
                                ),
                        )
                        .item(
                            El::<NodeBundle>::new()
                                .with_style(|style| {
                                    style.width = Val::Percent(40.);
                                    style.height = Val::Percent(100.);
                                })
                                .align_content(Align::center())
                                .child(
                                    Column::<NodeBundle>::new()
                                        .with_style(|style| {
                                            style.row_gap = Val::Px(CELL_GAP * 2.);
                                        })
                                        .item(cell().align(Align::center()))
                                        .item(arrow())
                                        .item(
                                            El::<NodeBundle>::new()
                                                .with_style(|style| style.width = Val::Px(CELL_WIDTH * 2. + CELL_GAP))
                                                .child(grid(4).align_content(Align::new().center_x())),
                                        ),
                                ),
                        ),
                )
                .item(
                    El::<NodeBundle>::new()
                        .with_style(|style| style.width = Val::Percent(100.))
                        .child(grid(27).align_content(Align::new().center_x())),
                )
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|style| {
                            style.column_gap = Val::Px(CELL_GAP);
                        })
                        .items((0..9).into_iter().map(|_| cell())),
                ),
        )
}

fn spawn_ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.0);
            style.height = Val::Percent(100.0);
        })
        .align_content(Align::center())
        .child(inventory())
        .spawn(world);
}
