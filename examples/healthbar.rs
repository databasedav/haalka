//! - Simple 3D Scene with a character (sphere).
//!   - Can be moved around with WASD/arrow keys.
//! - A health bar and character name is anchored to the character in world-space.
//! - The health starts at 10 and decreases by 1 every second. The health should be stored and
//!   managed in Bevy ECS. When reaching 0 HP, the character should be despawned together with UI.

mod utils;
use utils::*;

use bevy::prelude::*;
use colorgrad::{self, Gradient};
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(PreStartup, setup)
        .add_systems(Startup, |world: &mut World| {
            ui_root().spawn(world);
        })
        .add_systems(
            Update,
            (
                movement,
                sync_tracking_healthbar_position,
                decay,
                sync_health_mutable,
                despawn_when_dead,
            )
                .chain()
                .run_if(any_with_component::<Player>),
        )
        .add_observer(
            |_: Trigger<SpawnPlayer>,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             mut commands: Commands| {
                let health = Mutable::new(PLAYER_HEALTH);
                commands.spawn((
                    Player,
                    Health(PLAYER_HEALTH),
                    HealthMutable(health.clone()),
                    Mesh3d(meshes.add(Mesh::from(Sphere { radius: RADIUS }))),
                    Transform::from_translation(PLAYER_POSITION),
                    MeshMaterial3d(materials.add(Color::srgb_u8(228, 147, 58))),
                ));
                HEALTH_OPTION_MUTABLE.set(Some(health));
            },
        )
        .insert_resource(HealthTickTimer(Timer::from_seconds(
            HEALTH_TICK_RATE,
            TimerMode::Repeating,
        )))
        .run();
}

const SPEED: f32 = 10.0;
const RADIUS: f32 = 0.5;
const MINI: (f32, f32) = (200., 25.);
const MAXI: (f32, f32) = (500., 50.);
const NAME: &str = "avi";
const CAMERA_POSITION: Vec3 = Vec3::new(8., 10.5, 8.);
const PLAYER_POSITION: Vec3 = Vec3::new(0., RADIUS, 0.);
const PLAYER_HEALTH: u32 = 10;
const HEALTH_TICK_RATE: f32 = 1.;

#[derive(Clone, Copy, Default, PartialEq)]
struct StyleData {
    left: f32,
    top: f32,
    scale: f32,
}

static STYLE_DATA: LazyLock<Mutable<StyleData>> = LazyLock::new(default);

#[derive(Component)]
struct Health(u32);

#[derive(Component)]
struct HealthMutable(Mutable<u32>);

fn sync_health_mutable(health_query: Single<(&Health, &HealthMutable), Changed<Health>>) {
    let (health, health_mutable) = *health_query;
    health_mutable.0.set(health.0);
}

#[derive(Component)]
struct Player;

fn setup(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(87, 108, 50))),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 1_500_000.,
            range: 100.,
            ..default()
        },
        Transform::from_xyz(0., 8., 0.),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.trigger(SpawnPlayer);
}

fn movement(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<&Transform, (With<Camera3d>, Without<Player>)>,
    mut player: Single<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction += Vec3::from(camera.forward());
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction += Vec3::from(camera.left());
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction += Vec3::from(camera.back());
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction += Vec3::from(camera.right());
    }
    let movement = direction.normalize_or_zero() * SPEED * time.delta_secs();
    player.translation.x += movement.x;
    player.translation.z += movement.z;
}

#[allow(clippy::type_complexity)]
fn sync_tracking_healthbar_position(
    player_transform: Single<&Transform, (With<Player>, Changed<Transform>)>,
    camera_data: Single<(&Camera, &Transform), (With<Camera3d>, Without<Player>)>,
    // mut ui_scale: ResMut<UiScale>,  // wanted more local ui scaling
) {
    let (camera, camera_transform) = *camera_data;
    let scale = camera_transform.translation.distance(player_transform.translation);
    if let Ok((left, top)) = camera
        .world_to_viewport(&GlobalTransform::from(*camera_transform), player_transform.translation)
        .map(|p| p.into())
    {
        STYLE_DATA.set_neq(StyleData { left, top, scale });
    }
    // let starting_distance = CAMERA_POSITION.distance(PLAYER_POSITION);
    // ui_scale.0 = starting_distance as f64 / scale as f64;
}

fn healthbar(
    max: u32,
    health: impl Signal<Item = u32> + Send + Sync + 'static,
    height: f32,
    color_gradient: impl Gradient + Send + Sync + 'static,
) -> Stack<Node> {
    let health = health.broadcast();
    let percent_health = health.signal().map(move |h| h as f32 / max as f32).broadcast();
    Stack::<Node>::new()
        .height(Val::Px(height))
        .with_node(move |mut node| {
            node.border = UiRect::all(Val::Px(height / 12.));
        })
        .border_color(BorderColor(Color::BLACK))
        .layer(
            El::<Node>::new()
                .height(Val::Percent(100.))
                .width_signal(percent_health.signal().map(|ph| ph * 100.).map(Val::Percent))
                .background_color_signal(percent_health.signal().map(move |percent_health| {
                    let [r, g, b, ..] = color_gradient.at(percent_health).to_rgba8();
                    BackgroundColor(Color::srgb_u8(r, g, b))
                })),
        )
        .layer(
            // TODO: why is this wrapping node required? it wasn't required in 0.14
            El::<Node>::new()
                .height(Val::Percent(100.))
                .align_content(Align::new().center_y())
                .child(
                    El::<Text>::new()
                        // TODO: text should be centerable vertically via flex; https://github.com/bevyengine/bevy/issues/14266
                        // TODO: this align doesn't work
                        // .align(Align::new().center_y())
                        .with_node(move |mut node| {
                            node.top = Val::Px(height / 32.);
                            node.left = Val::Px(height / 6.); // TODO: padding doesn't work here?
                        })
                        .align(Align::new().left())
                        .text_font(TextFont::from_font_size(height))
                        .text_color(TextColor(Color::BLACK))
                        .text_signal(health.signal_ref(ToString::to_string).map(Text)),
                ),
        )
}

static HEALTH_OPTION_MUTABLE: LazyLock<Mutable<Option<Mutable<u32>>>> = LazyLock::new(default);

#[derive(Event)]
struct SpawnPlayer;

fn respawn_button() -> impl Element {
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
        .on_click_with_system(|_: In<_>, mut commands: Commands| commands.trigger(SpawnPlayer))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(50.))
                .text_color(TextColor(Color::WHITE))
                .text(Text::new("respawn")),
        )
}

fn set_dragging_position(mut node: Mut<Node>, StyleData { left, top, .. }: StyleData) {
    node.left = Val::Px(left - MINI.0 / 2.);
    node.top = Val::Px(top - 30. * 2. - MINI.1 * 1.5);
}

fn ui_root() -> impl Element {
    // let starting_distance = CAMERA_POSITION.distance(PLAYER_POSITION);
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .child_signal(
            HEALTH_OPTION_MUTABLE
                .signal_cloned()
                .map_option(
                    move |health| {
                        health
                            .signal()
                            .map(|health| health > 0)
                            .dedupe()
                            .map_bool(
                                move || {
                                    Stack::<Node>::new()
                                        .width(Val::Percent(100.))
                                        .height(Val::Percent(100.))
                                        .with_node(|mut node| node.padding.bottom = Val::Px(10.))
                                        .layer(
                                            Column::<Node>::new()
                                                .on_signal_with_node(STYLE_DATA.signal(), set_dragging_position)
                                                .with_node(|mut node| {
                                                    node.row_gap = Val::Px(MINI.1 / 4.);
                                                    set_dragging_position(node, STYLE_DATA.get());
                                                })
                                                // .on_signal_with_transform(style_data.signal(), move |transform,
                                                // StyleData { scale, .. }| {
                                                //     transform.scale = Vec3::splat(starting_distance / scale);
                                                // })
                                                .item(
                                                    El::<Text>::new()
                                                        .with_node(|mut node| node.left = Val::Px(MINI.1 / 4.))
                                                        .text_font(TextFont::from_font_size(MINI.1 * 3. / 4.))
                                                        .text_color(TextColor(Color::WHITE))
                                                        .text(Text::new(NAME)),
                                                )
                                                .item(
                                                    healthbar(
                                                        PLAYER_HEALTH,
                                                        health.signal(),
                                                        MINI.1,
                                                        colorgrad::GradientBuilder::new()
                                                            .html_colors(&["purple", "yellow"])
                                                            .build::<colorgrad::LinearGradient>()
                                                            .unwrap(),
                                                    )
                                                    .width(Val::Px(MINI.0))
                                                    .height(Val::Px(MINI.1)),
                                                ),
                                        )
                                        .layer(
                                            healthbar(
                                                PLAYER_HEALTH,
                                                health.signal(),
                                                MAXI.1,
                                                colorgrad::GradientBuilder::new()
                                                    .html_colors(&["red", "green"])
                                                    .build::<colorgrad::LinearGradient>()
                                                    .unwrap(),
                                            )
                                            .align(Align::new().bottom().center_x())
                                            .width(Val::Px(MAXI.0))
                                            .height(Val::Px(MAXI.1)),
                                        )
                                        .type_erase()
                                },
                                || respawn_button().type_erase(),
                            )
                            .boxed()
                    },
                    || always(respawn_button().type_erase()).boxed(),
                )
                .flatten(),
        )
}

#[derive(Resource)]
struct HealthTickTimer(Timer);

fn decay(mut health: Single<&mut Health>, mut health_tick_timer: ResMut<HealthTickTimer>, time: Res<Time>) {
    if health_tick_timer.0.tick(time.delta()).finished() {
        health.0 = health.0.saturating_sub(1);
    }
}

fn despawn_when_dead(mut commands: Commands, query: Single<(Entity, &Health), Changed<Health>>) {
    let (entity, health) = *query;
    if health.0 == 0 {
        commands.entity(entity).despawn();
    }
}
