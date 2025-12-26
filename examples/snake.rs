//! Snake with adjustable grid size and tick rate.

mod utils;
use utils::*;

use std::{collections::HashMap, time::Duration};

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
                .run_if(not(resource_exists::<Paused>)),)
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

#[derive(Resource, Clone, Copy, PartialEq, Deref, DerefMut)]
struct TickRate(u32);

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct Score(u32);

#[derive(Resource, Clone, Copy, PartialEq, Deref, DerefMut)]
struct GridSize(usize);

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct GameOver(bool);

#[derive(Resource, Clone)]
struct Cells(MutableVec<Cell>);

/// Convert (x, y) grid position to linear index in display order (top-left to bottom-right).
/// Display order is: row by row from top (y=size-1) to bottom (y=0), left (x=0) to right.
fn pos_to_index(x: usize, y: usize, size: usize) -> usize {
    (size - 1 - y) * size + x
}

fn init_cells(world: &mut World, size: usize) -> MutableVec<Cell> {
    let initial: Vec<Cell> = vec![Cell::Empty; size * size];
    MutableVecBuilder::from(initial).spawn(world)
}

fn grid(cells: MutableVec<Cell>) -> impl Element {
    let cell_size_signal = SignalBuilder::from_resource::<GridSize>()
        .map_in(deref_copied)
        .dedupe()
        // TODO: see https://github.com/bevyengine/bevy/issues/12152 for why this slack is necessary
        .map_in(|size| (SIDE as f32 - GRID_TRACK_FLOAT_PRECISION_SLACK) / size as f32);

    Grid::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(SIDE as f32);
            node.height = Val::Px(SIDE as f32);
        })
        .row_wrap_cell_width_signal(cell_size_signal.clone())
        .cells_signal_vec(
            cells
                .signal_vec()
                .map(|In(cell): In<Cell>| El::<Node>::new().background_color(BackgroundColor::from(cell))),
        )
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
                SignalBuilder::from_resource::<Score>()
                    .map_in(deref_copied)
                    .dedupe()
                    .map_in_ref(ToString::to_string)
                    .map_in(Text)
                    .map_in(Some),
            ),
        )
        .item(
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
                            SignalBuilder::from_resource::<GridSize>()
                                .map_in(deref_copied)
                                .dedupe()
                                .map_in_ref(ToString::to_string)
                                .map_in(Text)
                                .map_in(Some),
                        ),
                )
                .item(text_button("-").on_pressing_throttled(
                    |_: In<_>, mut commands: Commands| {
                        commands.trigger(GridSizeChange::Decr);
                    },
                    Duration::from_millis(100),
                ))
                .item(text_button("+").on_pressing_throttled(
                    |_: In<_>, mut commands: Commands| {
                        commands.trigger(GridSizeChange::Incr);
                    },
                    Duration::from_millis(100),
                )),
        )
        .item(
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
                            SignalBuilder::from_resource::<TickRate>()
                                .map_in(deref_copied)
                                .dedupe()
                                .map_in_ref(ToString::to_string)
                                .map_in(Text)
                                .map_in(Some),
                        ),
                )
                .item(text_button("-").on_pressing_throttled(
                    |_: In<_>, mut tick_rate: ResMut<TickRate>, mut commands: Commands| {
                        if **tick_rate > 1 {
                            **tick_rate -= 1;
                            commands.insert_resource(Time::<Fixed>::from_seconds(1. / **tick_rate as f64));
                        }
                    },
                    Duration::from_millis(100),
                ))
                .item(text_button("+").on_pressing_throttled(
                    |_: In<_>, mut tick_rate: ResMut<TickRate>, mut commands: Commands| {
                        **tick_rate += 1;
                        commands.insert_resource(Time::<Fixed>::from_seconds(1. / **tick_rate as f64));
                    },
                    Duration::from_millis(100),
                )),
        )
}

fn ui_root(cells: MutableVec<Cell>) -> impl Element {
    Stack::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .cursor(CursorIcon::default())
        .layer(Row::<Node>::new().align(Align::center()).item(grid(cells)).item(hud()))
        .layer_signal(
            SignalBuilder::from_resource::<GameOver>()
                .map_in(deref_copied)
                .dedupe()
                .map_true_in(|| restart_button().type_erase()),
        )
}

fn restart_button() -> impl Element {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert(Pickable::default())
        .align(Align::center())
        .with_node(|mut node| {
            node.width = Val::Px(250.);
            node.height = Val::Px(80.);
        })
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            SignalBuilder::from_lazy_entity(lazy_entity)
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

#[derive(Event, Clone, Copy)]
enum GridSizeChange {
    Incr,
    Decr,
}

fn on_grid_size_change(
    event: On<GridSizeChange>,
    cells: Res<Cells>,
    snake: Res<Snake>,
    mut grid_size: ResMut<GridSize>,
    mut vec_datas: Query<&mut MutableVecData<Cell>>,
    mut commands: Commands,
) {
    let event = *event;
    let cur_size = **grid_size;
    match event {
        GridSizeChange::Incr => {
            let new_size = cur_size + 1;
            let mut cells_write = cells.0.write(&mut vec_datas);

            // Insert new top row (new_size empty cells at indices 0..new_size-1)
            for i in 0..new_size {
                cells_write.insert(i, Cell::Empty);
            }

            // Insert new right column cell at end of each old row
            // After inserting top row, old row k's end is at index: new_size + (k+1)*cur_size + k - 1
            // We insert at: new_size + (k+1)*cur_size + k = new_size + k*new_size + cur_size
            for k in 0..cur_size {
                let insert_idx = new_size + k * new_size + cur_size;
                cells_write.insert(insert_idx, Cell::Empty);
            }

            **grid_size = new_size;
        }
        GridSizeChange::Decr => {
            if cur_size > 2 {
                let new_size = cur_size - 1;
                // Check if any snake segments are on the edges being removed
                let can_shrink = !snake.0.iter().any(|&(x, y)| x == new_size || y == new_size);

                if can_shrink {
                    let mut cells_write = cells.0.write(&mut vec_datas);

                    // Check if food is on the edges being removed (top row or right column)
                    let had_food = (0..cur_size).any(|i| {
                        // Top row: indices 0..cur_size
                        // Right column: indices cur_size-1, 2*cur_size-1, 3*cur_size-1, ...
                        let top_idx = i;
                        let right_idx = (i + 1) * cur_size - 1;
                        matches!(cells_write.get(top_idx), Some(Cell::Food))
                            || matches!(cells_write.get(right_idx), Some(Cell::Food))
                    });

                    // Remove from highest index to lowest to preserve indices
                    // Right column cells: (cur_size * cur_size - 1), minus cur_size each time
                    // For cur_size=4: indices 15, 11, 7 (but not 3, that's in top row)
                    for k in 0..new_size {
                        let remove_idx = cur_size * cur_size - 1 - k * cur_size;
                        cells_write.remove(remove_idx);
                    }

                    // Top row: indices cur_size-1 down to 0
                    for i in (0..cur_size).rev() {
                        cells_write.remove(i);
                    }

                    drop(cells_write);
                    **grid_size = new_size;

                    if had_food {
                        commands.trigger(SpawnFood);
                    }
                }
            }
        }
    }
}

fn text_button(text_: &str) -> impl Element + PointerEventAware {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert(Pickable::default())
        .with_node(|mut node| node.width = Val::Px(45.0))
        .align_content(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            SignalBuilder::from_lazy_entity(lazy_entity)
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
        Direction::Up => (x, if y == size - 1 { 0 } else { y + 1 }),
        Direction::Down => (x, y.checked_sub(1).unwrap_or(size - 1)),
        Direction::Left => (x.checked_sub(1).unwrap_or(size - 1), y),
        Direction::Right => (if x == size - 1 { 0 } else { x + 1 }, y),
    };
    snake.0.push_front((x, y));

    let head_idx = pos_to_index(x, y, size);
    let mut cells_write = cells.0.write(&mut vec_datas);
    let cell = cells_write[head_idx];

    match cell {
        Cell::Snake => {
            drop(cells_write);
            **game_over = true;
            commands.insert_resource(Paused);
        }
        cell @ (Cell::Food | Cell::Empty) => {
            cells_write.set(head_idx, Cell::Snake);
            match cell {
                Cell::Food => {
                    drop(cells_write);
                    **score += 1;
                    commands.trigger(SpawnFood);
                }
                Cell::Empty => {
                    if let Some((tail_x, tail_y)) = snake.0.pop_back() {
                        let tail_idx = pos_to_index(tail_x, tail_y, size);
                        cells_write.set(tail_idx, Cell::Empty);
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
    let mut cells_write = cells.0.write(&mut vec_datas);
    let empty_indices: Vec<_> = cells_write
        .iter()
        .enumerate()
        .filter_map(|(idx, cell)| matches!(cell, Cell::Empty).then_some(idx))
        .collect();

    if let Some(idx) = empty_indices.into_iter().choose(rng.as_mut()) {
        cells_write.set(idx, Cell::Food);
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

    // Create fresh grid with snake cells
    let mut new_cells = vec![Cell::Empty; size * size];
    for &(x, y) in init_snake.iter() {
        let idx = pos_to_index(x, y, size);
        new_cells[idx] = Cell::Snake;
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
    let map = HashMap::from([
        (KeyCode::KeyW, Direction::Up),
        (KeyCode::KeyA, Direction::Left),
        (KeyCode::KeyS, Direction::Down),
        (KeyCode::KeyD, Direction::Right),
        (KeyCode::ArrowUp, Direction::Up),
        (KeyCode::ArrowLeft, Direction::Left),
        (KeyCode::ArrowDown, Direction::Down),
        (KeyCode::ArrowRight, Direction::Right),
    ]);
    for (key, key_dir) in map.iter() {
        if keys.pressed(*key) {
            queued_direction_option.0 = Some(*key_dir);
            return;
        }
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
