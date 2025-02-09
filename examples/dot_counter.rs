//! Demonstrates how to forward ECS changes to UI.

mod utils;
use bevy_render::view::RenderLayers;
use utils::*;

use std::time::Duration;

use bevy::prelude::*;
use bevy_rand::prelude::*;
use haalka::prelude::*;
use rand::prelude::{IteratorRandom, Rng};

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<ChaCha8Rng>::default()))
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    let camera = world.spawn((Camera2d, IsDefaultUiCamera)).id();
                    ui_root()
                        .update_raw_el(|raw_el| {
                            raw_el.with_entity(move |mut entity| {
                                // https://github.com/bevyengine/bevy/discussions/11223
                                entity.insert(TargetCamera(camera));
                            })
                        })
                        .spawn(world);
                },
                dot_camera,
                |mut commands: Commands| {
                    commands.spawn((Spawner, MutableTimer::from(SPAWN_RATE.clone())));
                    commands.spawn((Despawner, MutableTimer::from(DESPAWN_RATE.clone())));
                },
            ),
        )
        .add_systems(
            Update,
            (tick_emitter::<Spawner, SpawnDot>, tick_emitter::<Despawner, DespawnDot>),
        )
        .add_observer(
            |_: Trigger<SpawnDot>,
             mut rng: GlobalEntropy<ChaCha8Rng>,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<ColorMaterial>>,
             mut commands: Commands| {
                let translation = Vec3::new(rng.gen::<f32>() * HEIGHT, rng.gen::<f32>() * HEIGHT, 0.)
                    - Vec3::new(WIDTH / 2., HEIGHT / 2., -1.);
                let color = position_to_color(translation);
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(10.))),
                    MeshMaterial2d(materials.add(ColorMaterial::from(Color::BLACK))),
                    Transform::from_translation(translation),
                    Dot(color),
                    RenderLayers::layer(1),
                ));
            },
        )
        .add_observer(
            |_: Trigger<DespawnDot>,
             dots: Query<Entity, With<Dot>>,
             mut rng: GlobalEntropy<ChaCha8Rng>,
             mut commands: Commands| {
                if let Some(dot) = dots.iter().choose(rng.as_mut()) {
                    commands.entity(dot).despawn_recursive();
                }
            },
        )
        .run();
}

#[derive(Event, Default)]
struct SpawnDot;

#[derive(Event, Default)]
struct DespawnDot;

#[derive(Clone, Copy)]
enum ColorCategory {
    Blue,
    Green,
    Red,
    Yellow,
}

const WIDTH: f32 = 1280.; // default window
const HEIGHT: f32 = 720.; // default window
const BOX_SIZE: f32 = HEIGHT / 2.;
const FONT_SIZE: f32 = 25.;

fn box_(category: ColorCategory) -> El<Node> {
    El::<Node>::new()
        .width(Val::Px(BOX_SIZE))
        .height(Val::Px(BOX_SIZE))
        .background_color(BackgroundColor(match category {
            ColorCategory::Blue => BLUE,
            ColorCategory::Green => GREEN,
            ColorCategory::Red => RED,
            ColorCategory::Yellow => YELLOW,
        }))
}

fn labeled_element(label: impl Element, element: impl Element) -> impl Element {
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(10.))
        .item(label)
        // TODO: vertical text layout regression https://github.com/bevyengine/bevy/issues/16627
        .item(element.align(Align::new().center_y()))
}

fn labeled_count(label: impl Element, count_signal: impl Signal<Item = i32> + Send + 'static) -> impl Element {
    labeled_element(label, {
        El::<Text>::new()
            .text_font(TextFont::from_font_size(FONT_SIZE))
            .text_signal(count_signal.map(|count| Text(count.to_string())))
    })
}

fn text_labeled_element(label: &str, element: impl Element) -> impl Element {
    labeled_element(
        El::<Text>::new()
            .text_font(TextFont::from_font_size(FONT_SIZE))
            .text(Text(format!("{}: ", label))),
        element,
    )
}

fn text_labeled_count(label: &str, count_signal: impl Signal<Item = i32> + Send + 'static) -> impl Element {
    text_labeled_element(label, {
        El::<Text>::new()
            .text_font(TextFont::from_font_size(FONT_SIZE))
            .text_signal(count_signal.map(|count| Text(count.to_string())))
    })
}

fn category_count(category: ColorCategory, count: impl Signal<Item = i32> + Send + 'static) -> impl Element {
    labeled_count(
        {
            El::<Node>::new()
                .width(Val::Px(30.))
                .height(Val::Px(30.))
                .background_color(BackgroundColor(match category {
                    ColorCategory::Blue => BLUE,
                    ColorCategory::Green => GREEN,
                    ColorCategory::Red => RED,
                    ColorCategory::Yellow => YELLOW,
                }))
                .align(Align::center())
        },
        count,
    )
}

// like serde
fn incrde_button<T: Component>(step: f32) -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map(Into::into),
        )
        .hovered_sync(hovered)
        .on_pressing_with_system_with_sleep_throttle(
            move |_: In<_>, mut timer: Single<&mut MutableTimer, With<T>>| {
                timer.incr(step);
            },
            Duration::from_millis(50),
        )
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(if step.is_sign_positive() { "+" } else { "-" })),
        )
}

fn rate_element<T: Component>(rate: Mutable<f32>) -> impl Element {
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(15.0))
        .item(incrde_button::<T>(0.1))
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(rate.signal().map(|rate| Text(format!("{:.1}", rate)))),
        )
        .item(incrde_button::<T>(-0.1))
}

#[derive(Component)]
struct MutableTimer {
    timer: Timer,
    rate: Mutable<f32>,
}

impl MutableTimer {
    fn from(rate: Mutable<f32>) -> Self {
        Self {
            timer: Timer::from_seconds(1. / rate.get(), TimerMode::Repeating),
            rate,
        }
    }

    fn incr(&mut self, step: f32) {
        let rate = self.rate.get();
        let new = (rate + step).max(0.);
        if new > 0. {
            self.timer.unpause();
            self.timer.set_duration(Duration::from_secs_f32(1. / new));
            self.rate.set(new);
        } else {
            self.timer.pause();
        }
    }
}

#[derive(Component)]
struct Spawner;

#[derive(Component)]
struct Despawner;

#[derive(Default)]
struct Counts {
    blue: Mutable<i32>,
    green: Mutable<i32>,
    red: Mutable<i32>,
    yellow: Mutable<i32>,
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
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .width(Val::Px(WIDTH))
                .height(Val::Px(HEIGHT))
                .with_node(|mut node| node.column_gap = Val::Px(50.))
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.width = Val::Px(2. * BOX_SIZE))
                        .item(
                            Row::<Node>::new()
                                .item(box_(ColorCategory::Blue))
                                .item(box_(ColorCategory::Green)),
                        )
                        .item(
                            Row::<Node>::new()
                                .item(box_(ColorCategory::Red))
                                .item(box_(ColorCategory::Yellow)),
                        ),
                )
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| {
                            node.row_gap = Val::Px(50.);
                            node.padding.left = Val::Px(50.);
                        })
                        .item(
                            Row::<Node>::new()
                                .item(
                                    Column::<Node>::new()
                                        .align_content(Align::new().left())
                                        .with_node(|mut node| node.row_gap = Val::Px(10.))
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
                                    .align(Align::new().center_x()),
                                ),
                        )
                        .item(
                            Column::<Node>::new()
                                .with_node(|mut node| node.row_gap = Val::Px(10.))
                                .item(text_labeled_element(
                                    "spawn rate",
                                    rate_element::<Spawner>(SPAWN_RATE.clone()),
                                ))
                                .item(text_labeled_element(
                                    "despawn rate",
                                    rate_element::<Despawner>(DESPAWN_RATE.clone()),
                                )),
                        ),
                ),
        )
}

const BLUE: Color = Color::srgb(0.25, 0.25, 0.75);
const GREEN: Color = Color::srgb(0.25, 0.75, 0.25);
const RED: Color = Color::srgb(0.75, 0.25, 0.25);
const YELLOW: Color = Color::srgb(0.75, 0.75, 0.25);

fn dot_camera(mut commands: Commands) {
    // https://github.com/bevyengine/bevy/discussions/11223
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(1),
    ));
}

#[derive(Clone, Copy, Component)]
#[component(on_add = incr_color_count, on_remove = decr_color_count)]
struct Dot(ColorCategory);

fn update_color_count(color: ColorCategory, step: i32) {
    let count = match color {
        ColorCategory::Blue => &COUNTS.blue,
        ColorCategory::Green => &COUNTS.green,
        ColorCategory::Red => &COUNTS.red,
        ColorCategory::Yellow => &COUNTS.yellow,
    };
    count.update(|count| count + step);
}

fn incr_color_count(world: bevy::ecs::world::DeferredWorld, entity: Entity, _: bevy::ecs::component::ComponentId) {
    if let Some(Dot(color)) = world.get::<Dot>(entity).copied() {
        update_color_count(color, 1);
    }
}

fn decr_color_count(world: bevy::ecs::world::DeferredWorld, entity: Entity, _: bevy::ecs::component::ComponentId) {
    if let Some(Dot(color)) = world.get::<Dot>(entity).copied() {
        update_color_count(color, -1);
    }
}

fn tick_emitter<T: Component, E: Event + Default>(
    mut spawner: Single<&mut MutableTimer, With<T>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if spawner.timer.tick(time.delta()).finished() {
        commands.trigger(E::default());
        spawner.timer.reset();
    }
}

fn position_to_color(position: Vec3) -> ColorCategory {
    let x = position.x + WIDTH / 2.0;
    let y = position.y + BOX_SIZE;

    if (0.0..BOX_SIZE).contains(&x) {
        if (0.0..BOX_SIZE).contains(&y) {
            return ColorCategory::Red;
        } else if (BOX_SIZE..2.0 * BOX_SIZE).contains(&y) {
            return ColorCategory::Blue;
        }
    } else if (BOX_SIZE..2.0 * BOX_SIZE).contains(&x) {
        if (0.0..BOX_SIZE).contains(&y) {
            return ColorCategory::Yellow;
        } else if (BOX_SIZE..2.0 * BOX_SIZE).contains(&y) {
            return ColorCategory::Green;
        }
    }
    panic!("Invalid position: {:?}", position);
}
