//! Snake with adjustable grid size and tick rate.

mod utils;
use utils::*;

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    convert::identity,
    time::Duration,
};

use bevy::prelude::*;
use bevy_rand::prelude::*;
use haalka::{grid::GRID_TRACK_FLOAT_PRECISION_SLACK, prelude::*};
use rand::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

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
                |mut restart: EventWriter<Restart>| {
                    restart.send_default();
                },
            ),
        )
        .add_systems(Update, (direction, restart.run_if(on_event::<Restart>())))
        .add_systems(
            FixedUpdate,
            (
                (
                    spawn_food.run_if(on_event::<SpawnFood>()),
                    consume_queued_direction,
                    tick,
                )
                    .chain()
                    .run_if(not(resource_exists::<Paused>)),
                grid_size_changer.run_if(on_event::<GridSizeChange>()),
            )
                .chain(),
        )
        .insert_resource(DirectionResource(Direction::Right))
        .insert_resource(Time::<Fixed>::from_seconds(1. / STARTING_TICKS_PER_SECOND as f64))
        .insert_resource(QueuedDirectionOption(None))
        .add_event::<SpawnFood>()
        .add_event::<GridSizeChange>()
        .add_event::<Restart>()
        .run();
}

const STARTING_SIZE: usize = 20;
const SIDE: usize = 720; // TODO: reactively auto fit to height
const WIDTH: usize = 1280; // TODO: reactively auto fit to height
const EMPTY_COLOR: Color = Color::srgb(91. / 255., 206. / 255., 250. / 255.);
const SNAKE_COLOR: Color = Color::srgb(245. / 255., 169. / 255., 184. / 255.);
const FOOD_COLOR: Color = Color::srgb(255. / 255., 255. / 255., 255. / 255.);
const STARTING_TICKS_PER_SECOND: u32 = 10;

#[derive(Resource)]
struct Paused;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Empty,
    Snake,
    Food,
}

impl Into<BackgroundColor> for Cell {
    fn into(self) -> BackgroundColor {
        match self {
            Cell::Empty => EMPTY_COLOR,
            Cell::Snake => SNAKE_COLOR,
            Cell::Food => FOOD_COLOR,
        }
        .into()
    }
}

static TICK_RATE: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(STARTING_TICKS_PER_SECOND));

static SCORE: Lazy<Mutable<u32>> = Lazy::new(default);

static GRID_SIZE: Lazy<Mutable<usize>> = Lazy::new(|| Mutable::new(STARTING_SIZE));

type CellsType = MutableBTreeMap<(usize, usize), Mutable<Cell>>;

static CELLS: Lazy<CellsType> = Lazy::new(|| {
    (0..STARTING_SIZE)
        .flat_map(|x| (0..STARTING_SIZE).map(move |y| ((x, y), Mutable::new(Cell::Empty))))
        .collect::<BTreeMap<_, _>>()
        .into()
});

fn grid(size: Mutable<usize>, cells: CellsType) -> impl Element {
    let cell_size = size
        .signal()
        // TODO: see https://github.com/bevyengine/bevy/issues/12152 for why this slack is necessary
        .map(|size| (SIDE as f32 - GRID_TRACK_FLOAT_PRECISION_SLACK) / size as f32)
        .broadcast();
    Grid::<NodeBundle>::new()
        .width(Val::Px(SIDE as f32))
        .height(Val::Px(SIDE as f32))
        .row_wrap_cell_width_signal(cell_size.signal())
        .cells_signal_vec(
            cells
                .entries_cloned()
                .sort_by_cloned(|(left, _), (right, _)| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)))
                .map(move |(_, cell)| {
                    El::<NodeBundle>::new()
                        .width_signal(cell_size.signal().map(Val::Px))
                        .height_signal(cell_size.signal().map(Val::Px))
                        .background_color_signal(cell.signal().dedupe().map(Into::into))
                }),
        )
}

fn hud(score: Mutable<u32>, size: Mutable<usize>, tick_rate: Mutable<u32>) -> impl Element {
    Column::<NodeBundle>::new()
        .width(Val::Px((WIDTH - SIDE) as f32))
        .with_style(|mut style| style.row_gap = Val::Px(10.))
        .align_content(Align::center())
        .item(El::<TextBundle>::new().text_signal(score.signal().map(|score| {
            Text::from_section(
                score.to_string(),
                TextStyle {
                    font_size: 300.,
                    ..default()
                },
            )
        })))
        .item(
            Row::<NodeBundle>::new()
                .with_style(|mut style| style.column_gap = Val::Px(10.))
                .item(El::<TextBundle>::new().text(text("grid size:")))
                .item(El::<TextBundle>::new().text_signal(size.signal().map(|size| text(&size.to_string()))))
                .item(text_button("-").on_pressing_with_system_with_sleep_throttle(
                    |_: In<_>, mut grid_size_changes: EventWriter<GridSizeChange>| {
                        grid_size_changes.send(GridSizeChange::Decr);
                    },
                    Duration::from_millis(100),
                ))
                .item(text_button("+").on_pressing_with_system_with_sleep_throttle(
                    |_: In<_>, mut grid_size_changes: EventWriter<GridSizeChange>| {
                        grid_size_changes.send(GridSizeChange::Incr);
                    },
                    Duration::from_millis(100),
                )),
        )
        .item(
            Row::<NodeBundle>::new()
                .with_style(|mut style| style.column_gap = Val::Px(10.))
                .item(El::<TextBundle>::new().text(text("tick rate:")))
                .item(El::<TextBundle>::new().text_signal(tick_rate.signal().map(|size| text(&size.to_string()))))
                .item(text_button("-").on_pressing_with_system_with_sleep_throttle(
                    |_: In<_>, world: &mut World| {
                        let cur_rate = TICK_RATE.get();
                        if cur_rate > 1 {
                            TICK_RATE.update(|rate| rate - 1);
                            world.insert_resource(Time::<Fixed>::from_seconds(1. / (cur_rate - 1) as f64));
                        }
                    },
                    Duration::from_millis(100),
                ))
                .item(text_button("+").on_pressing_with_system_with_sleep_throttle(
                    |_: In<_>, world: &mut World| {
                        let cur_rate = TICK_RATE.get();
                        TICK_RATE.update(|rate| rate + 1);
                        world.insert_resource(Time::<Fixed>::from_seconds(1. / (cur_rate + 1) as f64));
                    },
                    Duration::from_millis(100),
                )),
        )
}

fn ui_root() -> impl Element {
    Stack::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .layer(
            Row::<NodeBundle>::new()
                .align(Align::center())
                // .width(Val::Percent(100.))
                // .height(Val::Percent(100.))
                .item(grid(GRID_SIZE.clone(), CELLS.clone()))
                .item(hud(SCORE.clone(), GRID_SIZE.clone(), TICK_RATE.clone())),
        )
        .layer_signal(GAME_OVER.signal().dedupe().map_true(restart_button))
}

fn restart_button() -> impl Element {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
        .align(Align::center())
        .width(Val::Px(250.))
        .height(Val::Px(80.))
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .align_content(Align::center())
        .on_click(|| async_world().send_event(Restart).apply(spawn).detach())
        .child(El::<TextBundle>::new().text(Text::from_section(
            "restart",
            TextStyle {
                font_size: 60.,
                color: Color::WHITE,
                ..default()
            },
        )))
}

fn text(string: &str) -> Text {
    Text::from_section(
        string,
        TextStyle {
            font_size: 30.,
            ..default()
        },
    )
}

#[derive(Event)]
enum GridSizeChange {
    Incr,
    Decr,
}

// TODO: move this back inside the on_click ? (initial motivation for moving to event was
// potentially addressing the grid float precision shenanigans)
fn grid_size_changer(mut events: EventReader<GridSizeChange>, mut spawn_food: EventWriter<SpawnFood>) {
    for event in events.read() {
        let cur_size = GRID_SIZE.get();
        match event {
            GridSizeChange::Incr => {
                let mut cells_lock = CELLS.lock_mut();
                for i in 0..cur_size + 1 {
                    cells_lock.insert_cloned((i, cur_size), Mutable::new(Cell::Empty));
                    cells_lock.insert_cloned((cur_size, i), Mutable::new(Cell::Empty));
                }
                GRID_SIZE.update(|size| size + 1);
            }
            GridSizeChange::Decr => {
                if cur_size > 2 {
                    let mut cells_lock = CELLS.lock_mut();
                    let indices = (0..cur_size)
                        .map(|i| (i, cur_size - 1))
                        .chain((0..cur_size).map(|i| (cur_size - 1, i)))
                        .collect::<Vec<_>>();
                    if indices.iter().all(|index| {
                        cells_lock
                            .get(index)
                            .map(|cell| !matches!(cell.get(), Cell::Snake))
                            .unwrap_or(false)
                    }) {
                        let mut removed = vec![];
                        for index in indices {
                            removed.push(cells_lock.remove(&index));
                        }
                        if removed
                            .into_iter()
                            .filter_map(identity)
                            .any(|removed| matches!(removed.get(), Cell::Food))
                        {
                            spawn_food.send_default();
                        }
                        GRID_SIZE.update(|size| size - 1);
                    }
                }
            }
        }
    }
}

fn text_button(text_: &str) -> impl Element + PointerEventAware {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| SNAKE_COLOR, || EMPTY_COLOR)
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .child(El::<TextBundle>::new().text(text(text_)))
}

// u could also just scan the cells every tick, but i'm just caching it
#[derive(Resource)]
struct Snake(VecDeque<(usize, usize)>);

static GAME_OVER: Lazy<Mutable<bool>> = Lazy::new(default);

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
    mut commands: Commands,
    mut snake: ResMut<Snake>,
    direction: Res<DirectionResource>,
    mut spawn_food: EventWriter<SpawnFood>,
) {
    let (mut x, mut y) = snake.0.front().copied().unwrap();
    (x, y) = match direction.0 {
        Direction::Up => (x, if y == GRID_SIZE.get() - 1 { 0 } else { y + 1 }),
        Direction::Down => (x, y.checked_sub(1).unwrap_or_else(|| GRID_SIZE.get() - 1)),
        Direction::Left => (x.checked_sub(1).unwrap_or_else(|| GRID_SIZE.get() - 1), y),
        Direction::Right => (if x == GRID_SIZE.get() - 1 { 0 } else { x + 1 }, y),
    };
    snake.0.push_front((x, y));
    let cells_lock = CELLS.lock_ref();
    if let Some(new) = cells_lock.get(&(x, y)) {
        match new.get() {
            Cell::Snake => {
                GAME_OVER.set(true);
                commands.insert_resource(Paused);
            }
            cell @ (Cell::Food | Cell::Empty) => {
                new.set(Cell::Snake);
                match cell {
                    Cell::Food => {
                        SCORE.update(|score| score + 1);
                        spawn_food.send_default();
                    }
                    Cell::Empty => {
                        if let Some((x, y)) = snake.0.pop_back() {
                            if let Some(cell) = cells_lock.get(&(x, y)) {
                                cell.set(Cell::Empty);
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

#[derive(Event, Default)]
struct SpawnFood;

fn spawn_food(mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    let cells_lock = CELLS.lock_ref();
    let empty_cells = cells_lock
        .iter()
        .filter_map(|(position, cell)| matches!(cell.get(), Cell::Empty).then_some(position));
    cells_lock
        .get(&empty_cells.choose(&mut *rng).unwrap())
        .unwrap()
        .set(Cell::Food);
}

#[derive(Event, Default)]
struct Restart;

fn restart(
    mut commands: Commands,
    mut spawn_food: EventWriter<SpawnFood>,
    mut queued_direction_option: ResMut<QueuedDirectionOption>,
    mut direction: ResMut<DirectionResource>,
) {
    for (_, cell) in CELLS.lock_ref().iter() {
        cell.set(Cell::Empty);
    }
    let size = GRID_SIZE.get();
    let init_snake = vec![(size / 2, size / 2 - 1), (size / 2 - 1, size / 2 - 1)];
    let cells_lock = CELLS.lock_ref();
    for (x, y) in init_snake.iter() {
        cells_lock.get(&(*x, *y)).unwrap().set_neq(Cell::Snake);
    }
    commands.insert_resource(Snake(VecDeque::from(init_snake)));
    queued_direction_option.0 = None;
    direction.0 = Direction::Right;
    spawn_food.send_default();
    SCORE.set_neq(0);
    GAME_OVER.set_neq(false);
    commands.remove_resource::<Paused>();
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
    commands.spawn(Camera2dBundle::default());
}
