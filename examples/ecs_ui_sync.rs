//! Demonstrates how to forward ECS changes to UI.

mod utils;
use utils::*;

use std::time::Duration;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rand::prelude::*;
use haalka::prelude::*;
use rand::prelude::{IteratorRandom, Rng};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(example_window()),
            HaalkaPlugin,
            EntropyPlugin::<ChaCha8Rng>::default(),
            FpsOverlayPlugin,
        ))
        .add_systems(Startup, (|world: &mut World| { ui_root().spawn(world); }, setup))
        .add_systems(Update, (sync_timer, dot_spawner, dot_despawner))
        .insert_resource(Spawner(MutableTimer::from(SPAWN_RATE.clone())))
        .insert_resource(Despawner(MutableTimer::from(DESPAWN_RATE.clone())))
        .run();
}

enum ColorCategory {
    Blue,
    Green,
    Red,
    Yellow,
}

const WIDTH: f32 = 1280.; // default window
const HEIGHT: f32 = 720.; // default window
const BOX_SIZE: f32 = HEIGHT / 2.;
const FONT_SIZE: f32 = 30.;

#[allow(dead_code)]
fn box_(category: ColorCategory) -> El<NodeBundle> {
    El::<NodeBundle>::new()
        .width(Val::Px(BOX_SIZE))
        .height(Val::Px(BOX_SIZE))
        .background_color(BackgroundColor(match category {
            ColorCategory::Blue => BLUE,
            ColorCategory::Green => GREEN,
            ColorCategory::Red => RED,
            ColorCategory::Yellow => YELLOW,
        }))
        .align(Align::center())
    // .child(El::<TextBundle>::new().text(text(&category.to_string())))
}

fn text(string: &str) -> Text {
    Text::from_section(
        string,
        TextStyle {
            font_size: FONT_SIZE,
            ..default()
        },
    )
}

fn labeled_element(label: impl Element, element: impl Element) -> impl Element {
    Row::<NodeBundle>::new()
        .with_style(|mut style| style.column_gap = Val::Px(10.))
        .item(label)
        .item(element)
}

fn labeled_count(label: impl Element, count_signal: impl Signal<Item = u32> + Send + 'static) -> impl Element {
    labeled_element(label, {
        El::<TextBundle>::new().text_signal(count_signal.map(|count| text(&count.to_string())))
    })
}

fn text_labeled_element(label: &str, element: impl Element) -> impl Element {
    labeled_element(El::<TextBundle>::new().text(text(&format!("{}: ", label))), element)
}

fn text_labeled_count(label: &str, count_signal: impl Signal<Item = u32> + Send + 'static) -> impl Element {
    text_labeled_element(label, {
        El::<TextBundle>::new().text_signal(count_signal.map(|count| text(&count.to_string())))
    })
}

fn category_count(category: ColorCategory, count: impl Signal<Item = u32> + Send + 'static) -> impl Element {
    labeled_count(
        {
            El::<NodeBundle>::new()
                .width(Val::Px(30.))
                .height(Val::Px(30.))
                .background_color(BackgroundColor(match category {
                    ColorCategory::Blue => BLUE,
                    ColorCategory::Green => GREEN,
                    ColorCategory::Red => RED,
                    ColorCategory::Yellow => YELLOW,
                }))
                .align(Align::center())
            // .child(El::<TextBundle>::new().text(text(&category.to_string())))
        },
        count,
    )
}

// like serde
fn incrde_button(value: Mutable<f32>, incr: f32) -> impl Element {
    let hovered = Mutable::new(false);
    let f = move || {
        let new = (*value.lock_ref() + incr).max(0.);
        *value.lock_mut() = new;
    };
    El::<NodeBundle>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .on_pressing_with_sleep_throttle(f, Duration::from_millis(50))
        .child(El::<TextBundle>::new().text(text(if incr.is_sign_positive() { "+" } else { "-" })))
}

fn rate_element(rate: Mutable<f32>) -> impl Element {
    Row::<NodeBundle>::new()
        .with_style(|mut style| style.column_gap = Val::Px(15.0))
        .item(El::<TextBundle>::new().text_signal(rate.signal().map(|rate| text(&format!("{:.1}", rate)))))
        .item(incrde_button(rate.clone(), 0.1))
        .item(incrde_button(rate, -0.1))
}

struct MutableTimer {
    timer: Timer,
    rate: Mutable<f32>,
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.000001
}

impl MutableTimer {
    fn from(rate: Mutable<f32>) -> Self {
        Self {
            timer: Timer::from_seconds(1. / rate.get(), TimerMode::Repeating),
            rate,
        }
    }

    fn sync(&mut self) {
        let rate = self.rate.get();
        if rate > 0. {
            self.timer.unpause();
            let new = 1. / rate;
            let cur = self.timer.duration().as_secs_f32();
            if !close(new, cur) {
                self.timer.set_duration(Duration::from_secs_f32(new));
            }
        } else {
            self.timer.pause();
        }
    }
}

#[derive(Resource)]
struct Spawner(MutableTimer);

#[derive(Resource)]
struct Despawner(MutableTimer);

#[derive(Default)]
struct Counts {
    blue: Mutable<u32>,
    green: Mutable<u32>,
    red: Mutable<u32>,
    yellow: Mutable<u32>,
}

const STARTING_SPAWN_RATE: f32 = 1.5;
const STARTING_DESPAWN_RATE: f32 = 1.;

static SPAWN_RATE: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(STARTING_SPAWN_RATE));
static DESPAWN_RATE: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(STARTING_DESPAWN_RATE));
static COUNTS: Lazy<Counts> = Lazy::new(default);

fn ui_root() -> impl Element {
    let counts = MutableVec::new_with_values(vec![
        COUNTS.blue.clone(),
        COUNTS.green.clone(),
        COUNTS.red.clone(),
        COUNTS.yellow.clone(),
    ]);
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<NodeBundle>::new()
                .with_style(|mut style| style.column_gap = Val::Px(50.))
                .item(
                    El::<NodeBundle>::new().width(Val::Px(HEIGHT)).height(Val::Px(HEIGHT)), // can't put non ui nodes on top of ui nodes; yes u can https://discord.com/channels/691052431525675048/743663673393938453/1192729978744352858
                                                                                            // Column::<NodeBundle>::new()
                                                                                            // .with_z_index(|z_index| *z_index = ZIndex::Global(1))
                                                                                            // .item(Row::<NodeBundle>::new().item(box_(Category::A)).item(box_(Category::B)))
                                                                                            // .item(Row::<NodeBundle>::new().item(box_(Category::C)).item(box_(Category::D)))
                )
                .item(
                    Column::<NodeBundle>::new()
                        .with_style(|mut style| {
                            style.row_gap = Val::Px(50.);
                            style.padding.left = Val::Px(50.);
                        })
                        .item(
                            Row::<NodeBundle>::new()
                                .item(
                                    Column::<NodeBundle>::new()
                                        .align_content(Align::new().left())
                                        .with_style(|mut style| style.row_gap = Val::Px(10.))
                                        .item(category_count(ColorCategory::Blue, COUNTS.blue.signal()))
                                        .item(category_count(ColorCategory::Green, COUNTS.green.signal()))
                                        .item(category_count(ColorCategory::Red, COUNTS.red.signal()))
                                        .item(category_count(ColorCategory::Yellow, COUNTS.yellow.signal())),
                                )
                                .item(
                                    text_labeled_count("total", {
                                        counts
                                            .signal_vec_cloned()
                                            .map_signal(|count| count.signal())
                                            .to_signal_map(|counts| counts.iter().sum())
                                            .dedupe()
                                    })
                                    .align(Align::new().right())
                                    .update_raw_el(|raw_el| {
                                        raw_el.with_component::<Style>(|mut style| style.right = Val::Px(100.))
                                    }),
                                ),
                        )
                        .item(
                            Column::<NodeBundle>::new()
                                .with_style(|mut style| style.row_gap = Val::Px(10.))
                                .item(text_labeled_element("spawn rate", rate_element(SPAWN_RATE.clone())))
                                .item(text_labeled_element("despawn rate", rate_element(DESPAWN_RATE.clone()))),
                        ),
                ),
        )
}

const BLUE: Color = Color::srgb(0.25, 0.25, 0.75);
const GREEN: Color = Color::srgb(0.25, 0.75, 0.25);
const RED: Color = Color::srgb(0.75, 0.25, 0.25);
const YELLOW: Color = Color::srgb(0.75, 0.75, 0.25);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color: BLUE,
            custom_size: Some(Vec2::new(BOX_SIZE, BOX_SIZE)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-WIDTH / 2. + BOX_SIZE / 2., BOX_SIZE / 2., 0.)),
        ..default()
    },));
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color: GREEN,
            custom_size: Some(Vec2::new(BOX_SIZE, BOX_SIZE)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-WIDTH / 2. + BOX_SIZE * 3. / 2., BOX_SIZE / 2., 0.)),
        ..default()
    },));
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color: RED,
            custom_size: Some(Vec2::new(BOX_SIZE, BOX_SIZE)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-WIDTH / 2. + BOX_SIZE / 2., -BOX_SIZE / 2., 0.)),
        ..default()
    },));
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color: YELLOW,
            custom_size: Some(Vec2::new(BOX_SIZE, BOX_SIZE)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-WIDTH / 2. + BOX_SIZE * 3. / 2., -BOX_SIZE / 2., 0.)),
        ..default()
    },));
}

fn sync_timer(mut spawner: ResMut<Spawner>, mut despawner: ResMut<Despawner>) {
    // TODO: just replace the timer resource with async_world instead, communicating in the ui ->
    // ecs world with mutables is an anti pattern, the other direction is fine tho
    spawner.0.sync();
    despawner.0.sync();
}

#[derive(Component)]
struct Dot;

// TODO: use global async world on click to send such events
// #[derive(Event)]
// struct SpawnDot;

// #[derive(Event)]
// struct DepawnDot;

// fn spawn_dot() {}
// fn despawn_dot() {}

fn dot_spawner(
    mut commands: Commands,
    mut spawner: ResMut<Spawner>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>,
) {
    if spawner.0.timer.tick(time.delta()).finished() {
        let translation = Vec3::new(rng.gen::<f32>() * HEIGHT, rng.gen::<f32>() * HEIGHT, 0.)
            - Vec3::new(WIDTH / 2., HEIGHT / 2., -1.);
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(Circle::new(10.)).into(),
                material: materials.add(ColorMaterial::from(Color::BLACK)),
                transform: Transform::from_translation(translation),
                ..default()
            },
            Dot,
        ));
        let count = match position_to_color(translation) {
            ColorCategory::Blue => &COUNTS.blue,
            ColorCategory::Green => &COUNTS.green,
            ColorCategory::Red => &COUNTS.red,
            ColorCategory::Yellow => &COUNTS.yellow,
        };
        count.update(|count| count + 1);
        spawner.0.timer.reset();
    }
}

fn dot_despawner(
    mut commands: Commands,
    mut despawner: ResMut<Despawner>,
    time: Res<Time>,
    dots: Query<(Entity, &Transform), With<Dot>>,
    mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>,
) {
    if despawner.0.timer.tick(time.delta()).finished() {
        if let Some((dot, transform)) = dots.iter().choose(rng.as_mut()) {
            commands.entity(dot).despawn_recursive();
            let count = match position_to_color(transform.translation) {
                ColorCategory::Blue => &COUNTS.blue,
                ColorCategory::Green => &COUNTS.green,
                ColorCategory::Red => &COUNTS.red,
                ColorCategory::Yellow => &COUNTS.yellow,
            };
            count.update(|count| count - 1);
        }
        despawner.0.timer.reset();
    }
}

fn position_to_color(position: Vec3) -> ColorCategory {
    let x = position.x + WIDTH / 2.0;
    let y = position.y + BOX_SIZE;

    if x >= 0.0 && x < BOX_SIZE {
        if y >= 0.0 && y < BOX_SIZE {
            return ColorCategory::Red;
        } else if y >= BOX_SIZE && y < 2.0 * BOX_SIZE {
            return ColorCategory::Blue;
        }
    } else if x >= BOX_SIZE && x < 2.0 * BOX_SIZE {
        if y >= 0.0 && y < BOX_SIZE {
            return ColorCategory::Yellow;
        } else if y >= BOX_SIZE && y < 2.0 * BOX_SIZE {
            return ColorCategory::Green;
        }
    }
    panic!("Invalid position: {:?}", position);
}
