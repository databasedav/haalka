use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    convert::identity,
};

use bevy::prelude::*;
use bevy_rand::prelude::*;
use haalka::*;
use rand::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

fn main() {
    let cells = (0..STARTING_SIZE)
        .flat_map(|x| (0..STARTING_SIZE).map(move |y| ((x, y), Mutable::new(Cell::Empty))))
        .collect::<BTreeMap<_, _>>();
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
            EntropyPlugin::<ChaCha8Rng>::default(),
        ))
        .add_systems(
            Startup,
            (ui_root, camera, |mut restart: EventWriter<Restart>| {
                restart.send_default();
            }),
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
        .insert_resource(GridSize(Mutable::new(STARTING_SIZE)))
        .insert_resource(Cells(cells.into()))
        .insert_resource(DirectionResource(Direction::Right))
        .insert_resource(TickRate(Mutable::new(STARTING_TICKS_PER_SECOND)))
        .insert_resource(Time::<Fixed>::from_seconds(1. / STARTING_TICKS_PER_SECOND as f64))
        .insert_resource(Score(Mutable::new(0)))
        .insert_resource(QueuedDirectionOption(None))
        .insert_resource(GameOver(Mutable::new(false)))
        .add_event::<SpawnFood>()
        .add_event::<GridSizeChange>()
        .add_event::<Restart>()
        .run();
}

const STARTING_SIZE: usize = 20;
const SIDE: usize = 720; // TODO: reactively auto fit to height
const WIDTH: usize = 1280; // TODO: reactively auto fit to height
const EMPTY_COLOR: Color = Color::rgb(91. / 255., 206. / 255., 250. / 255.);
const SNAKE_COLOR: Color = Color::rgb(245. / 255., 169. / 255., 184. / 255.);
const FOOD_COLOR: Color = Color::rgb(255. / 255., 255. / 255., 255. / 255.);
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

#[derive(Resource)]
struct TickRate(Mutable<u32>);

#[derive(Resource)]
struct Score(Mutable<u32>);

#[derive(Resource)]
struct GridSize(Mutable<usize>);

type CellsType = MutableBTreeMap<(usize, usize), Mutable<Cell>>;

#[derive(Resource)]
struct Cells(CellsType);

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
        .with_style(|style| style.row_gap = Val::Px(10.))
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
                .with_style(|style| style.column_gap = Val::Px(10.))
                .item(El::<TextBundle>::new().text(text("grid size:")))
                .item(El::<TextBundle>::new().text_signal(size.signal().map(|size| text(&size.to_string()))))
                .item(text_button("-", || {
                    spawn(async_world().send_event(GridSizeChange::Decr)).detach()
                }))
                .item(text_button("+", || {
                    spawn(async_world().send_event(GridSizeChange::Incr)).detach()
                })),
        )
        .item(
            Row::<NodeBundle>::new()
                .with_style(|style| style.column_gap = Val::Px(10.))
                .item(El::<TextBundle>::new().text(text("tick rate:")))
                .item(El::<TextBundle>::new().text_signal(tick_rate.signal().map(|size| text(&size.to_string()))))
                .item(text_button("-", || {
                    spawn(async_world().apply(|world: &mut World| {
                        let tick_rate = &world.resource::<TickRate>().0;
                        let cur_rate = tick_rate.get();
                        if cur_rate > 1 {
                            tick_rate.update(|rate| rate - 1);
                            world.insert_resource(Time::<Fixed>::from_seconds(1. / (cur_rate - 1) as f64));
                        }
                    }))
                    .detach()
                }))
                .item(text_button("+", || {
                    spawn(async_world().apply(|world: &mut World| {
                        let tick_rate = &world.resource::<TickRate>().0;
                        let cur_rate = tick_rate.get();
                        tick_rate.update(|rate| rate + 1);
                        world.insert_resource(Time::<Fixed>::from_seconds(1. / (cur_rate + 1) as f64));
                    }))
                    .detach()
                })),
        )
}

fn ui_root(world: &mut World) {
    let size = world.resource::<GridSize>().0.clone();
    let cells = world.resource::<Cells>().0.clone();
    let score = world.resource::<Score>().0.clone();
    let tick_rate = world.resource::<TickRate>().0.clone();
    let game_over = world.resource::<GameOver>().0.clone();
    Stack::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .layer(
            Row::<NodeBundle>::new()
                .width(Val::Percent(100.))
                .height(Val::Percent(100.))
                .item(grid(size.clone(), cells))
                .item(hud(score, size, tick_rate)),
        )
        .layer_signal(game_over.signal().dedupe().map_true(restart_button))
        .spawn(world);
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
                .map_bool(|| Color::GRAY, || Color::BLACK)
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .align_content(Align::center())
        .on_click(|| spawn(async_world().send_event(Restart)).detach())
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
fn grid_size_changer(
    mut events: EventReader<GridSizeChange>,
    size: Res<GridSize>,
    cells: Res<Cells>,
    mut spawn_food: EventWriter<SpawnFood>,
) {
    for event in events.read() {
        let cur_size = size.0.get();
        match event {
            GridSizeChange::Incr => {
                let mut cells_lock = cells.0.lock_mut();
                for i in 0..cur_size + 1 {
                    cells_lock.insert_cloned((i, cur_size), Mutable::new(Cell::Empty));
                    cells_lock.insert_cloned((cur_size, i), Mutable::new(Cell::Empty));
                }
                size.0.update(|size| size + 1);
            }
            GridSizeChange::Decr => {
                if cur_size > 2 {
                    let mut cells_lock = cells.0.lock_mut();
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
                        size.0.update(|size| size - 1);
                    }
                }
            }
        }
    }
}

fn text_button(text_: &str, on_click: impl FnMut() + Send + Sync + 'static) -> impl Element {
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
        .on_click(on_click)
        .child(El::<TextBundle>::new().text(text(text_)))
}

// u could also just scan the cells every tick, but i'm just caching it
#[derive(Resource)]
struct Snake(VecDeque<(usize, usize)>);

#[derive(Resource)]
struct GameOver(Mutable<bool>);

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
    cells: Res<Cells>,
    mut snake: ResMut<Snake>,
    size: Res<GridSize>,
    direction: Res<DirectionResource>,
    score: Res<Score>,
    mut spawn_food: EventWriter<SpawnFood>,
    game_over: Res<GameOver>,
) {
    let (mut x, mut y) = snake.0.front().copied().unwrap();
    (x, y) = match direction.0 {
        Direction::Up => (x, if y == size.0.get() - 1 { 0 } else { y + 1 }),
        Direction::Down => (x, y.checked_sub(1).unwrap_or_else(|| size.0.get() - 1)),
        Direction::Left => (x.checked_sub(1).unwrap_or_else(|| size.0.get() - 1), y),
        Direction::Right => (if x == size.0.get() - 1 { 0 } else { x + 1 }, y),
    };
    snake.0.push_front((x, y));
    let cells_lock = cells.0.lock_ref();
    if let Some(new) = cells_lock.get(&(x, y)) {
        match new.get() {
            Cell::Snake => {
                game_over.0.set(true);
                commands.insert_resource(Paused);
            }
            cell @ (Cell::Food | Cell::Empty) => {
                new.set(Cell::Snake);
                match cell {
                    Cell::Food => {
                        score.0.update(|score| score + 1);
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

fn spawn_food(cells: Res<Cells>, mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    let cells_lock = cells.0.lock_ref();
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
    cells: Res<Cells>,
    grid_size: Res<GridSize>,
    game_over: Res<GameOver>,
    mut spawn_food: EventWriter<SpawnFood>,
    score: Res<Score>,
    mut queued_direction_option: ResMut<QueuedDirectionOption>,
    mut direction: ResMut<DirectionResource>,
) {
    for (_, cell) in cells.0.lock_ref().iter() {
        cell.set(Cell::Empty);
    }
    let size = grid_size.0.get();
    let init_snake = vec![(size / 2, size / 2 - 1), (size / 2 - 1, size / 2 - 1)];
    let cells_lock = cells.0.lock_ref();
    for (x, y) in init_snake.iter() {
        cells_lock.get(&(*x, *y)).unwrap().set_neq(Cell::Snake);
    }
    commands.insert_resource(Snake(VecDeque::from(init_snake)));
    queued_direction_option.0 = None;
    direction.0 = Direction::Right;
    spawn_food.send_default();
    score.0.set_neq(0);
    game_over.0.set_neq(false);
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
