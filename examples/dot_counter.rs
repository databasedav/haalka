//! Demonstrates how to forward ECS changes to UI.

mod utils;
use utils::*;

use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use bevy::{camera::visibility::RenderLayers, ecs::lifecycle::HookContext, prelude::*};
use bevy_rand::prelude::*;
use haalka::prelude::*;
use rand::prelude::{IteratorRandom, Rng};

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<WyRand>::default()))
        .insert_resource(BlueDotCount::default())
        .insert_resource(GreenDotCount::default())
        .insert_resource(RedDotCount::default())
        .insert_resource(YellowDotCount::default())
        .insert_resource(SpawnRate::default())
        .insert_resource(DespawnRate::default())
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
                },
                ui_camera,
                dot_camera,
                |spawn_rate: Res<SpawnRate>, despawn_rate: Res<DespawnRate>, mut commands: Commands| {
                    commands.spawn((Spawner, RateTimer::from(**spawn_rate)));
                    commands.spawn((Despawner, RateTimer::from(**despawn_rate)));
                },
            ),
        )
        .add_systems(
            Update,
            (tick_emitter::<Spawner, SpawnDot>, tick_emitter::<Despawner, DespawnDot>),
        )
        .add_observer(
            |_: On<SpawnDot>,
             mut rng: Single<&mut WyRand, With<GlobalRng>>,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<ColorMaterial>>,
             mut cached: Local<Option<(Handle<Mesh>, Handle<ColorMaterial>)>>,
             mut commands: Commands| {
                let (mesh, material) = cached.get_or_insert_with(|| {
                    (
                        meshes.add(Circle::new(10.)),
                        materials.add(ColorMaterial::from(Color::BLACK)),
                    )
                });
                let translation = Vec3::new(rng.random::<f32>() * HEIGHT, rng.random::<f32>() * HEIGHT, 0.)
                    - Vec3::new(WIDTH / 2., HEIGHT / 2., -1.);
                let color = position_to_color(translation);
                commands.spawn((
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(material.clone()),
                    Transform::from_translation(translation),
                    Dot(color),
                    RenderLayers::layer(1),
                ));
            },
        )
        .add_observer(
            |_: On<DespawnDot>,
             dots: Query<Entity, With<Dot>>,
             mut rng: Single<&mut WyRand, With<GlobalRng>>,
             mut commands: Commands| {
                if let Some(dot) = dots.iter().choose(rng.as_mut()) {
                    commands.entity(dot).despawn();
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
        .with_node(|mut node| {
            node.width = Val::Px(BOX_SIZE);
            node.height = Val::Px(BOX_SIZE);
        })
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
            .text_signal(
                count_signal
                    .dedupe()
                    .map_in_ref(ToString::to_string)
                    .map_in(Text)
                    .map_in(Some),
            )
    })
}

fn text_labeled_element(label: &str, element: impl Element) -> impl Element {
    labeled_element(
        El::<Text>::new()
            .text_font(TextFont::from_font_size(FONT_SIZE))
            .text(Text(format!("{label}:"))),
        element,
    )
}

fn text_labeled_count(
    label: &str,
    font_size: f32,
    count_signal: impl Signal<Item = i32> + Send + 'static,
) -> impl Element {
    labeled_element(
        El::<Text>::new()
            .text_font(TextFont::from_font_size(font_size))
            .text(Text(format!("{label}:"))),
        El::<Text>::new()
            .text_font(TextFont::from_font_size(font_size))
            .text_signal(
                count_signal
                    .dedupe()
                    .map_in_ref(ToString::to_string)
                    .map_in(Text)
                    .map_in(Some),
            ),
    )
}

fn category_count(category: ColorCategory, count: impl Signal<Item = i32> + Send + 'static) -> impl Element {
    labeled_count(
        {
            El::<Node>::new()
                .with_node(|mut node| {
                    node.width = Val::Px(30.);
                    node.height = Val::Px(30.);
                })
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
fn incrde_button<T: Component, R>(step: f32) -> impl Element
where
    R: Resource + Clone + Copy + Deref<Target = f32> + DerefMut,
{
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .with_node(|mut node| node.width = Val::Px(45.0))
        .insert((Pickable::default(), Hoverable))
        .align_content(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            signal::from_entity(lazy_entity)
                .has_component::<Hovered>()
                .dedupe()
                .map_bool_in(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .on_pressed_throttled(
            move |In((_, press_data)): In<(Entity, PressData)>,
                  mut rate: ResMut<R>,
                  mut timer: Single<&mut RateTimer, With<T>>| {
                if press_data.pressed {
                    let new_rate = (**rate + step).max(0.);
                    if new_rate != **rate {
                        timer.set_rate(new_rate);
                        **rate = new_rate;
                    }
                }
            },
            Duration::from_millis(100),
        )
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(if step.is_sign_positive() { "+" } else { "-" })),
        )
}

fn rate_element<T: Component, R>(rate_signal: impl Signal<Item = f32> + Send + 'static) -> impl Element
where
    R: Resource + Clone + Copy + Deref<Target = f32> + DerefMut,
{
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(15.0))
        .item(incrde_button::<T, R>(0.1))
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(
                    rate_signal
                        .dedupe()
                        .map_in(|rate| format!("{rate:.1}"))
                        .map_in(Text)
                        .map_in(Some),
                ),
        )
        .item(incrde_button::<T, R>(-0.1))
}

#[derive(Component)]
struct RateTimer {
    timer: Timer,
}

impl RateTimer {
    fn from(rate: f32) -> Self {
        Self {
            timer: Timer::from_seconds(1. / rate, TimerMode::Repeating),
        }
    }

    fn set_rate(&mut self, rate: f32) {
        if rate > 0. {
            self.timer.unpause();
            self.timer.set_duration(Duration::from_secs_f32(1. / rate));
        } else {
            self.timer.pause();
        }
    }
}

#[derive(Component)]
struct Spawner;

#[derive(Component)]
struct Despawner;

const STARTING_SPAWN_RATE: f32 = 1.5;
const STARTING_DESPAWN_RATE: f32 = 1.;

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct BlueDotCount(i32);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct GreenDotCount(i32);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct RedDotCount(i32);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct YellowDotCount(i32);

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct SpawnRate(f32);

impl Default for SpawnRate {
    fn default() -> Self {
        Self(STARTING_SPAWN_RATE)
    }
}

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct DespawnRate(f32);

impl Default for DespawnRate {
    fn default() -> Self {
        Self(STARTING_DESPAWN_RATE)
    }
}

fn color_boxes() -> impl Element {
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
        )
}

fn counts_panel() -> impl Element {
    let blue = signal::from_resource_changed::<BlueDotCount>().map_in(deref_copied);
    let green = signal::from_resource_changed::<GreenDotCount>().map_in(deref_copied);
    let red = signal::from_resource_changed::<RedDotCount>().map_in(deref_copied);
    let yellow = signal::from_resource_changed::<YellowDotCount>().map_in(deref_copied);
    Stack::<Node>::new()
        .layer(
            Column::<Node>::new()
                .align_content(Align::new().left())
                .with_node(|mut node| node.row_gap = Val::Px(10.))
                .item(category_count(ColorCategory::Blue, blue.clone()))
                .item(category_count(ColorCategory::Green, green.clone()))
                .item(category_count(ColorCategory::Red, red.clone()))
                .item(category_count(ColorCategory::Yellow, yellow.clone())),
        )
        .layer(
            text_labeled_count("total", 40., signal::sum!(blue, green, red, yellow).dedupe())
                .with_builder(|builder| builder.with_component::<Node>(|mut node| node.padding.right = Val::Px(30.)))
                .align(Align::new().right().center_y()),
        )
}

fn rates_panel() -> impl Element {
    Column::<Node>::new()
        .with_node(|mut node| node.row_gap = Val::Px(10.))
        .item(text_labeled_element(
            "spawn rate",
            rate_element::<Spawner, SpawnRate>(signal::from_resource_changed::<SpawnRate>().map_in(deref_copied)),
        ))
        .item(text_labeled_element(
            "despawn rate",
            rate_element::<Despawner, DespawnRate>(signal::from_resource_changed::<DespawnRate>().map_in(deref_copied)),
        ))
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .cursor(CursorIcon::default())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| {
                    node.width = Val::Px(WIDTH);
                    node.height = Val::Px(HEIGHT);
                    node.column_gap = Val::Px(50.);
                })
                .item(color_boxes())
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| {
                            node.row_gap = Val::Px(50.);
                            node.padding.left = Val::Px(50.);
                        })
                        .item(counts_panel())
                        .item(rates_panel()),
                ),
        )
}

const BLUE: Color = Color::srgb(0.25, 0.25, 0.75);
const GREEN: Color = Color::srgb(0.25, 0.75, 0.25);
const RED: Color = Color::srgb(0.75, 0.25, 0.25);
const YELLOW: Color = Color::srgb(0.75, 0.75, 0.25);

fn ui_camera(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
}

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

fn update_color_count(color: ColorCategory, step: i32, world: &mut bevy::ecs::world::DeferredWorld) {
    match color {
        ColorCategory::Blue => **world.resource_mut::<BlueDotCount>() += step,
        ColorCategory::Green => **world.resource_mut::<GreenDotCount>() += step,
        ColorCategory::Red => **world.resource_mut::<RedDotCount>() += step,
        ColorCategory::Yellow => **world.resource_mut::<YellowDotCount>() += step,
    }
}

fn incr_color_count(mut world: bevy::ecs::world::DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Dot(color) = *world.get::<Dot>(entity).unwrap();
    update_color_count(color, 1, &mut world);
}

fn decr_color_count(mut world: bevy::ecs::world::DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Dot(color) = *world.get::<Dot>(entity).unwrap();
    update_color_count(color, -1, &mut world);
}

fn tick_emitter<'a, T: Component, E: Event + Default>(
    mut spawner: Single<&mut RateTimer, With<T>>,
    time: Res<Time>,
    mut commands: Commands,
) where
    <E as bevy::prelude::Event>::Trigger<'a>: std::default::Default,
{
    if spawner.timer.tick(time.delta()).is_finished() {
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
    panic!("Invalid position: {position:?}");
}
