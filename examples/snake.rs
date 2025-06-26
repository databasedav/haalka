//! Snake with adjustable grid size and tick rate.

mod utils;
use utils::*;

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    time::Duration,
};

use bevy::prelude::*;
use bevy_rand::prelude::*;
use haalka::{grid::GRID_TRACK_FLOAT_PRECISION_SLACK, prelude::*};
use rand::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<ChaCha8Rng>::default()))
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

static TICK_RATE: LazyLock<Mutable<u32>> = LazyLock::new(|| Mutable::new(STARTING_TICKS_PER_SECOND));

static SCORE: LazyLock<Mutable<u32>> = LazyLock::new(default);

static GRID_SIZE: LazyLock<Mutable<usize>> = LazyLock::new(|| Mutable::new(STARTING_SIZE));

type CellsType = MutableBTreeMap<(usize, usize), Mutable<Cell>>;

static CELLS: LazyLock<CellsType> = LazyLock::new(|| {
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
    Grid::<Node>::new()
        .width(Val::Px(SIDE as f32))
        .height(Val::Px(SIDE as f32))
        .row_wrap_cell_width_signal(cell_size.signal())
        .cells_signal_vec(
            cells
                .entries_cloned()
                .sort_by_cloned(|(left, _), (right, _)| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)))
                .map(move |(_, cell)| {
                    El::<Node>::new()
                        .width_signal(cell_size.signal().map(Val::Px))
                        .height_signal(cell_size.signal().map(Val::Px))
                        .background_color_signal(cell.signal().dedupe().map(Into::<BackgroundColor>::into))
                }),
        )
}

fn hud(score: Mutable<u32>, size: Mutable<usize>, tick_rate: Mutable<u32>) -> impl Element {
    Column::<Node>::new()
        .width(Val::Px((WIDTH - SIDE) as f32))
        .with_node(|mut node| node.row_gap = Val::Px(10.))
        .align_content(Align::center())
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(250.))
                .text_signal(score.signal_ref(ToString::to_string).map(Text)),
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
                        .text_signal(size.signal_ref(ToString::to_string).map(Text)),
                )
                .item(text_button("-").on_pressing_with_system_with_sleep_throttle(
                    |_: In<_>, mut commands: Commands| {
                        commands.trigger(GridSizeChange::Decr);
                    },
                    Duration::from_millis(100),
                ))
                .item(text_button("+").on_pressing_with_system_with_sleep_throttle(
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
                        .text_signal(tick_rate.signal_ref(ToString::to_string).map(Text)),
                )
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
    Stack::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .layer(
            Row::<Node>::new()
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
    El::<Node>::new()
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
        .on_click_with_system(|_: In<_>, mut commands: Commands| commands.trigger(Restart))
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

fn on_grid_size_change(event: Trigger<GridSizeChange>, mut commands: Commands) {
    let event = *event;
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
                        .flatten()
                        .any(|removed| matches!(removed.get(), Cell::Food))
                    {
                        commands.trigger(SpawnFood);
                    }
                    GRID_SIZE.update(|size| size - 1);
                }
            }
        }
    }
}

fn text_button(text_: &str) -> impl Element + PointerEventAware {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| SNAKE_COLOR, || EMPTY_COLOR)
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(text_)),
        )
}

// u could also just scan the cells every tick, but i'm just caching it
#[derive(Resource)]
struct Snake(VecDeque<(usize, usize)>);

static GAME_OVER: LazyLock<Mutable<bool>> = LazyLock::new(default);

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

fn tick(mut snake: ResMut<Snake>, direction: Res<DirectionResource>, mut commands: Commands) {
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
                        commands.trigger(SpawnFood);
                    }
                    Cell::Empty => {
                        if let Some((x, y)) = snake.0.pop_back()
                            && let Some(cell) = cells_lock.get(&(x, y))
                        {
                            cell.set(Cell::Empty);
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

fn on_spawn_food(_: Trigger<SpawnFood>, mut rng: GlobalEntropy<ChaCha8Rng>) {
    let cells_lock = CELLS.lock_ref();
    let empty_cells = cells_lock
        .iter()
        .filter_map(|(position, cell)| matches!(cell.get(), Cell::Empty).then_some(position));
    cells_lock
        .get(empty_cells.choose(rng.as_mut()).unwrap())
        .unwrap()
        .set(Cell::Food);
}

#[derive(Event, Default)]
struct Restart;

fn on_restart(_: Trigger<Restart>, mut commands: Commands) {
    for (_, cell) in CELLS.lock_ref().iter() {
        cell.set(Cell::Empty);
    }
    let size = GRID_SIZE.get();
    let init_snake = vec![(size / 2, size / 2 - 1), (size / 2 - 1, size / 2 - 1)];
    let cells_lock = CELLS.lock_ref();
    for &(x, y) in init_snake.iter() {
        cells_lock.get(&(x, y)).unwrap().set_neq(Cell::Snake);
    }
    commands.insert_resource(Snake(VecDeque::from(init_snake)));
    commands.insert_resource(QueuedDirectionOption(None));
    commands.insert_resource(DirectionResource(Direction::Right));
    commands.trigger(SpawnFood);
    commands.remove_resource::<Paused>();
    SCORE.set_neq(0);
    GAME_OVER.set_neq(false);
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
