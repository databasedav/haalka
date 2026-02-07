//! Snake with adjustable grid size and tick rate.

mod utils;
use utils::*;

use std::time::Duration;

use bevy::prelude::*;
use bevy_rand::prelude::*;
use haalka::{grid::GRID_TRACK_FLOAT_PRECISION_SLACK, prelude::*};
use rand::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<WyRand>::default()))
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    let cells = init_cells(world, STARTING_SIZE);
                    world.insert_resource(Cells(cells.clone()));
                    ui_root(cells).spawn(world);
                },
                camera,
                |mut commands: Commands| commands.trigger(Restart),
            ),
        )
        .add_systems(Update, direction)
        .add_systems(
            FixedUpdate,
            ((consume_queued_direction, tick)
                .chain()
                .run_if(not(resource_exists::<Paused>)))
            .chain(),
        )
        .insert_resource(DirectionResource(Direction::Right))
        .insert_resource(Time::<Fixed>::from_seconds(1. / STARTING_TICKS_PER_SECOND as f64))
        .insert_resource(QueuedDirectionOption(None))
        .insert_resource(TickRate(STARTING_TICKS_PER_SECOND))
        .insert_resource(Score(0))
        .insert_resource(GridSize(STARTING_SIZE))
        .insert_resource(GameOver(false))
        .add_observer(on_restart)
        .add_observer(on_spawn_food)
        .add_observer(on_grid_size_change)
        .run();
}

const STARTING_SIZE: usize = 20;
const SIDE: usize = 720; // TODO: reactively auto fit to height
const WIDTH: usize = 1280; // TODO: reactively auto fit to height
const EMPTY_COLOR: Color = Color::srgb(91. / 255., 206. / 255., 250. / 255.);
const SNAKE_COLOR: Color = Color::srgb(245. / 255., 169. / 255., 184. / 255.);
const FOOD_COLOR: Color = Color::srgb(1., 1., 1.);
const STARTING_TICKS_PER_SECOND: u32 = 10;
const FONT_SIZE: f32 = 25.;

#[derive(Resource)]
struct Paused;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    Snake,
    Food,
}

impl From<Cell> for BackgroundColor {
    fn from(val: Cell) -> Self {
        match val {
            Cell::Empty => EMPTY_COLOR,
            Cell::Snake => SNAKE_COLOR,
            Cell::Food => FOOD_COLOR,
        }
        .into()
    }
}

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct TickRate(u32);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct Score(u32);

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct GridSize(usize);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct GameOver(bool);

#[derive(Resource, Clone)]
struct Cells(MutableVec<Cell>);

/// Convert (x, y) grid position to linear index in the flattened 1D cells array.
///
/// Grid coordinates: x increases right (0 to size-1), y increases down (0 to size-1).
/// Array layout: index 0 is top-left, then proceeds row-by-row left-to-right, top-to-bottom.
/// Example for 3x3 grid:
///   indices:  0 1 2    coords: (0,0) (1,0) (2,0)
///             3 4 5            (0,1) (1,1) (2,1)
///             6 7 8            (0,2) (1,2) (2,2)
fn pos_to_index(x: usize, y: usize, size: usize) -> usize {
    y * size + x
}

fn init_cells(world: &mut World, size: usize) -> MutableVec<Cell> {
    let initial: Vec<Cell> = vec![Cell::Empty; size * size];
    MutableVec::builder().values(initial).spawn(world)
}

fn grid(cells: MutableVec<Cell>) -> impl Element {
    let cell_size = signal::from_resource_changed::<GridSize>()
        .map_in(deref_copied)
        // TODO: see https://github.com/bevyengine/bevy/issues/12152 for why this slack is necessary
        .map_in(|size| (SIDE as f32 - GRID_TRACK_FLOAT_PRECISION_SLACK) / size as f32);

    Grid::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(SIDE as f32);
            node.height = Val::Px(SIDE as f32);
        })
        .row_wrap_cell_width_signal(cell_size.clone())
        .cells_signal_vec(
            cells
                .signal_vec()
                .map_in(|cell| El::<Node>::new().background_color(BackgroundColor::from(cell))),
        )
}

fn grid_size_control() -> impl Element {
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(10.))
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new("grid size:")),
        )
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(
                    signal::from_resource_changed::<GridSize>()
                        .map_in(deref_copied)
                        .map_in_ref(ToString::to_string)
                        .map_in(Text)
                        .map_in(Some),
                ),
        )
        .item(text_button("-").on_pressed_throttled(
            |In((_, press_data)): In<(Entity, PressData)>, mut commands: Commands| {
                if press_data.pressed {
                    commands.trigger(GridSizeChange::Decr);
                }
            },
            Duration::from_millis(100),
        ))
        .item(text_button("+").on_pressed_throttled(
            |In((_, press_data)): In<(Entity, PressData)>, mut commands: Commands| {
                if press_data.pressed {
                    commands.trigger(GridSizeChange::Incr);
                }
            },
            Duration::from_millis(100),
        ))
}

fn tick_rate_control() -> impl Element {
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(10.))
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new("tick rate:")),
        )
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(
                    signal::from_resource_changed::<TickRate>()
                        .map_in(deref_copied)
                        .map_in_ref(ToString::to_string)
                        .map_in(Text)
                        .map_in(Some),
                ),
        )
        .item(text_button("-").on_pressed_throttled(
            |In((_, press_data)): In<(Entity, PressData)>, mut tick_rate: ResMut<TickRate>, mut commands: Commands| {
                if press_data.pressed && **tick_rate > 1 {
                    **tick_rate -= 1;
                    commands.insert_resource(Time::<Fixed>::from_seconds(1. / **tick_rate as f64));
                }
            },
            Duration::from_millis(100),
        ))
        .item(text_button("+").on_pressed_throttled(
            |In((_, press_data)): In<(Entity, PressData)>, mut tick_rate: ResMut<TickRate>, mut commands: Commands| {
                if press_data.pressed {
                    **tick_rate += 1;
                    commands.insert_resource(Time::<Fixed>::from_seconds(1. / **tick_rate as f64));
                }
            },
            Duration::from_millis(100),
        ))
}

fn hud() -> impl Element {
    Column::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px((WIDTH - SIDE) as f32);
            node.row_gap = Val::Px(10.);
        })
        .align_content(Align::center())
        .item(
            El::<Text>::new().text_font(TextFont::from_font_size(250.)).text_signal(
                signal::from_resource_changed::<Score>()
                    .map_in(deref_copied)
                    .map_in_ref(ToString::to_string)
                    .map_in(Text)
                    .map_in(Some),
            ),
        )
        .item(grid_size_control())
        .item(tick_rate_control())
}

fn ui_root(cells: MutableVec<Cell>) -> impl Element {
    Stack::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .layer(Row::<Node>::new().align(Align::center()).item(grid(cells)).item(hud()))
        .layer_signal(
            signal::from_resource_changed::<GameOver>()
                .map_in(deref_copied)
                .map_true_in(restart_button),
        )
}

fn restart_button() -> impl Element + Clone {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert((Pickable::default(), Hoverable))
        .align(Align::center())
        .with_node(|mut node| {
            node.width = Val::Px(250.);
            node.height = Val::Px(80.);
        })
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            signal::from_entity(lazy_entity)
                .has_component::<Hovered>()
                .dedupe()
                .map_bool_in(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .align_content(Align::center())
        .on_click(|_: In<_>, mut commands: Commands| commands.trigger(Restart))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(50.))
                .text_color(TextColor(Color::WHITE))
                .text(Text::new("restart")),
        )
}

/// Event for dynamically resizing the grid during gameplay.
///
/// The grid uses a 1D array to store cells in row-major order (see [`pos_to_index`]).
/// Resizing requires careful index manipulation to maintain the visual layout.
#[derive(Event, Clone, Copy)]
enum GridSizeChange {
    /// Grow the grid by one row (at top) and one column (on right).
    Incr,
    /// Shrink the grid by removing the top row and right column.
    /// Only succeeds if the snake doesn't occupy those edges.
    Decr,
}

/// Handles grid resize events, adjusting the cell array and snake coordinates.
///
/// # Grid Layout Reminder
/// The cells array is stored in row-major order:
/// - Index 0 is top-left corner
/// - Indices increase left-to-right, then top-to-bottom
///
/// # Coordinate System
/// - `x` increases rightward (column index)
/// - `y` increases downward (row index)
fn on_grid_size_change(
    event: On<GridSizeChange>,
    cells: Res<Cells>,
    mut snake: ResMut<Snake>,
    mut grid_size: ResMut<GridSize>,
    mut vec_datas: Query<&mut MutableVecData<Cell>>,
    mut commands: Commands,
) {
    let event = *event;
    let cur_size = **grid_size;
    match event {
        GridSizeChange::Incr => {
            // Strategy: Add new row at top and new column on right
            //
            // Example: 3x3 -> 4x4
            //
            //   Before:       After:
            //    0 1 2        [.]  [.]  [.]  [.]
            //    3 4 5         4    5    6   [.]
            //    6 7 8         8    9   10   [.]
            //                 12   13   14   [.]
            //
            let new_size = cur_size + 1;
            let mut guard = cells.0.write(&mut vec_datas);

            // Insert entire new top row at indices 0..new_size.
            // Each insert at index i shifts existing elements right by 1.
            for i in 0..new_size {
                guard.insert(i, Cell::Empty);
            }

            // Insert new right column cell at the end of each original row.
            //
            // After inserting the top row (new_size cells), original row k is now at:
            //   start_of_row_k = new_size + k * cur_size + k
            //                               ^^^^^^^^^^^^   ^
            //                               original pos   cells we've inserted so far
            //
            // Insert position (after last cell of row k):
            //   insert_i = start_of_row_k + cur_size = new_size + (k + 1) * cur_size + k
            for k in 0..cur_size {
                let insert_i = new_size + (k + 1) * cur_size + k;
                guard.insert(insert_i, Cell::Empty);
            }

            // Update snake coordinates: adding top row shifts all y-coordinates down by 1
            for (_, y) in snake.0.iter_mut() {
                *y += 1;
            }

            **grid_size = new_size;
        }
        GridSizeChange::Decr => {
            // Minimum grid size is 2x2 (need room for snake to move)
            if cur_size <= 2 {
                return;
            }

            // Strategy: Remove top row and rightmost column
            //
            // Example: 4x4 -> 3x3
            //
            //   Before:                   After:
            //   [x] [x] [x] [x]
            //    4   5   6  [x]           0  1  2
            //    8   9  10  [x]           3  4  5
            //   12  13  14  [x]           6  7  8
            //
            let new_size = cur_size - 1;

            // Safety check: snake must not occupy edges being removed.
            //   Top row: y == 0
            //   Right column: x == cur_size - 1 (which equals new_size)
            let snake_on_removed_edge = snake.0.iter().any(|&(x, y)| x == new_size || y == 0);
            if snake_on_removed_edge {
                return;
            }

            let mut guard = cells.0.write(&mut vec_datas);

            // Check if food will be removed (need to respawn it afterward).
            // Food could be on:
            //   - Top row: indices 0, 1, ..., cur_size-1
            //   - Right column: indices cur_size-1, 2*cur_size-1, ..., cur_size*cur_size-1
            // Note: top-right corner (index cur_size-1) is counted in both, but that's fine.
            let had_food = (0..cur_size).any(|i| {
                let top_row_i = i;
                let right_col_i = (i + 1) * cur_size - 1;
                matches!(guard.get(top_row_i), Some(Cell::Food))
                    || matches!(guard.get(right_col_i), Some(Cell::Food))
            });

            // Remove right column cells (bottom to top to preserve indices).
            // For 4x4: remove indices 15, 11, 7 (not 3 - that's in top row, removed next)
            for k in 0..new_size {
                let remove_i = cur_size * cur_size - 1 - k * cur_size;
                guard.remove(remove_i);
            }

            // Remove entire top row (right to left to preserve indices).
            // For 4x4: remove indices 3, 2, 1, 0
            for i in (0..cur_size).rev() {
                guard.remove(i);
            }

            **grid_size = new_size;

            // Update snake coordinates (top row removed, so all y values shift up by 1)
            for (_, y) in snake.0.iter_mut() {
                *y -= 1;
            }

            // Respawn food if it was on a removed edge
            if had_food {
                commands.trigger(SpawnFood);
            }
        }
    }
}

fn text_button(text_: &str) -> impl Element + PointerEventAware {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert((Pickable::default(), Hoverable))
        .with_node(|mut node| node.width = Val::Px(45.0))
        .align_content(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            signal::from_entity(lazy_entity)
                .has_component::<Hovered>()
                .dedupe()
                .map_bool_in(|| SNAKE_COLOR, || EMPTY_COLOR)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(text_)),
        )
}

// u could also just scan the cells every tick, but i'm just caching it
#[derive(Resource)]
struct Snake(std::collections::VecDeque<(usize, usize)>);

#[derive(Clone, Copy, EnumIter, PartialEq, Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(&self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Resource)]
struct DirectionResource(Direction);

#[allow(clippy::too_many_arguments)]
fn tick(
    mut snake: ResMut<Snake>,
    direction: Res<DirectionResource>,
    grid_size: Res<GridSize>,
    cells: Res<Cells>,
    mut score: ResMut<Score>,
    mut game_over: ResMut<GameOver>,
    mut vec_datas: Query<&mut MutableVecData<Cell>>,
    mut commands: Commands,
) {
    let (mut x, mut y) = snake.0.front().copied().unwrap();
    let size = **grid_size;
    (x, y) = match direction.0 {
        Direction::Up => (x, y.checked_sub(1).unwrap_or(size - 1)),
        Direction::Down => (x, if y == size - 1 { 0 } else { y + 1 }),
        Direction::Left => (x.checked_sub(1).unwrap_or(size - 1), y),
        Direction::Right => (if x == size - 1 { 0 } else { x + 1 }, y),
    };
    snake.0.push_front((x, y));

    let head_i = pos_to_index(x, y, size);
    let mut guard = cells.0.write(&mut vec_datas);
    let cell = guard[head_i];

    match cell {
        Cell::Snake => {
            **game_over = true;
            commands.insert_resource(Paused);
        }
        cell @ (Cell::Food | Cell::Empty) => {
            guard.set(head_i, Cell::Snake);
            match cell {
                Cell::Food => {
                    **score += 1;
                    commands.trigger(SpawnFood);
                }
                Cell::Empty => {
                    if let Some((tail_x, tail_y)) = snake.0.pop_back() {
                        let tail_i = pos_to_index(tail_x, tail_y, size);
                        guard.set(tail_i, Cell::Empty);
                    }
                }
                _ => (),
            }
        }
    }
}

#[derive(Event, Default)]
struct SpawnFood;

fn on_spawn_food(
    _: On<SpawnFood>,
    cells: Res<Cells>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut vec_datas: Query<&mut MutableVecData<Cell>>,
) {
    let mut guard = cells.0.write(&mut vec_datas);
    if let Some(i) = guard
        .iter()
        .enumerate()
        .filter_map(|(i, cell)| matches!(cell, Cell::Empty).then_some(i))
        .into_iter()
        .choose(rng.as_mut())
    {
        guard.set(i, Cell::Food);
    }
}

#[derive(Event, Default)]
struct Restart;

fn on_restart(
    _: On<Restart>,
    cells: Res<Cells>,
    grid_size: Res<GridSize>,
    mut score: ResMut<Score>,
    mut game_over: ResMut<GameOver>,
    mut vec_datas: Query<&mut MutableVecData<Cell>>,
    mut commands: Commands,
) {
    let size = **grid_size;
    let init_snake = vec![(size / 2, size / 2 - 1), (size / 2 - 1, size / 2 - 1)];

    let mut new_cells = vec![Cell::Empty; size * size];
    for &(x, y) in init_snake.iter() {
        let i = pos_to_index(x, y, size);
        new_cells[i] = Cell::Snake;
    }
    cells.0.write(&mut vec_datas).replace(new_cells);

    commands.insert_resource(Snake(std::collections::VecDeque::from(init_snake)));
    commands.insert_resource(QueuedDirectionOption(None));
    commands.insert_resource(DirectionResource(Direction::Right));
    commands.trigger(SpawnFood);
    commands.remove_resource::<Paused>();

    if **score != 0 {
        **score = 0;
    }
    if **game_over {
        **game_over = false;
    }
}

#[derive(Resource)]
struct QueuedDirectionOption(Option<Direction>);

fn direction(keys: ResMut<ButtonInput<KeyCode>>, mut queued_direction_option: ResMut<QueuedDirectionOption>) {
    let dir = if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        Some(Direction::Up)
    } else if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        Some(Direction::Down)
    } else if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        Some(Direction::Left)
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        Some(Direction::Right)
    } else {
        None
    };

    if let Some(key_dir) = dir {
        queued_direction_option.0 = Some(key_dir);
    }
}

fn consume_queued_direction(
    mut queued_direction_option: ResMut<QueuedDirectionOption>,
    mut cur_dir: ResMut<DirectionResource>,
) {
    if let Some(queued_direction) = queued_direction_option.0.take() {
        for direction in Direction::iter() {
            if cur_dir.0 == direction && cur_dir.0.opposite() == queued_direction {
                return;
            }
        }
        cur_dir.0 = queued_direction;
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
