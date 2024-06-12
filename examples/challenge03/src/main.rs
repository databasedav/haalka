// Simple 3D Scene with a character (sphere).
//     Can be moved around with WASD/arrow keys.
// A health bar and character name is anchored to the character in world-space.
// The health starts at 10 and decreases by 1 every second. The health should be stored and managed
//     in Bevy ECS.
// When reaching 0 HP, the character should be despawned together with UI.

use bevy::prelude::*;
use colorgrad::{self, Gradient};
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
        .add_systems(PreStartup, setup)
        .add_systems(Startup, ui_root)
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
        .add_systems(Update, spawn_player.run_if(on_event::<SpawnPlayer>()))
        .insert_resource(StyleDataResource::default())
        .insert_resource(HealthTickTimer(Timer::from_seconds(
            HEALTH_TICK_RATE,
            TimerMode::Repeating,
        )))
        .insert_resource(HealthOptionMutable(default()))
        .add_event::<SpawnPlayer>()
        .run();
}

const SPEED: f32 = 10.0;
const RADIUS: f32 = 0.5;
const MINI: (f32, f32) = (200., 30.);
const MAXI: (f32, f32) = (500., 60.);
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

#[derive(Resource, Default)]
struct StyleDataResource(Mutable<StyleData>);

#[derive(Component)]
struct Health(u32);

#[derive(Component)]
struct HealthMutable(Mutable<u32>);

fn sync_health_mutable(health_query: Query<(&Health, &HealthMutable), Changed<Health>>) {
    if let Ok((health, health_mutable)) = health_query.get_single() {
        health_mutable.0.set(health.0);
    }
}

#[derive(Component)]
struct Player;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawn_player: EventWriter<SpawnPlayer>,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
        material: materials.add(Color::rgb_u8(87, 108, 50)),
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            intensity: 1_500_000.,
            range: 100.,
            ..default()
        },
        transform: Transform::from_xyz(0., 8., 0.),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(CAMERA_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    spawn_player.send_default();
}

fn movement(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Query<&Transform, (With<Camera3d>, Without<Player>)>,
    mut player: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;
    let mut player = player.single_mut();
    let camera = camera.single();
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
    let movement = direction.normalize_or_zero() * SPEED * time.delta_seconds();
    player.translation.x += movement.x;
    player.translation.z += movement.z;
}

fn sync_tracking_healthbar_position(
    style_data_resource: Res<StyleDataResource>,
    player: Query<&Transform, (With<Player>, Changed<Transform>)>,
    camera: Query<(&Camera, &Transform), (With<Camera3d>, Without<Player>)>,
    // mut ui_scale: ResMut<UiScale>,  // wanted more local ui scaling
) {
    let (camera, camera_transform) = camera.single();
    let player_transform = player.single();
    let scale = camera_transform.translation.distance(player_transform.translation);
    if let Some((left, top)) = camera
        .world_to_viewport(&GlobalTransform::from(*camera_transform), player_transform.translation)
        .map(|p| p.into())
    {
        style_data_resource.0.set_neq(StyleData { left, top, scale });
    }
    // let starting_distance = CAMERA_POSITION.distance(PLAYER_POSITION);
    // ui_scale.0 = starting_distance as f64 / scale as f64;
}

fn healthbar(
    max: u32,
    health: impl Signal<Item = u32> + Send + Sync + 'static,
    height: f32,
    color_gradient: Gradient,
) -> Stack<NodeBundle> {
    let health = health.broadcast();
    let percent_health = health.signal().map(move |h| h as f32 / max as f32).broadcast();
    Stack::<NodeBundle>::new()
        .height(Val::Px(height))
        .with_style(move |style| {
            style.border = UiRect::all(Val::Px(height / 12.));
        })
        .border_color(BorderColor(Color::BLACK))
        .layer(
            El::<NodeBundle>::new()
                .height(Val::Percent(100.))
                .width_signal(percent_health.signal().map(|ph| ph * 100.).map(Val::Percent))
                .background_color_signal(percent_health.signal().map(move |percent_health| {
                    let [r, g, b, ..] = color_gradient.at(percent_health as f64).to_rgba8();
                    Color::rgb_u8(r, g, b).into()
                })),
        )
        .layer(
            El::<TextBundle>::new()
                .height(Val::Percent(100.))
                .with_style(move |style| {
                    style.bottom = Val::Px(height / 8.);
                    style.left = Val::Px(height / 6.); // TODO: padding doesn't work here?
                })
                .align(Align::new().left())
                .text_signal(health.signal().map(move |health| {
                    Text::from_section(
                        health.to_string(),
                        TextStyle {
                            font_size: height,
                            color: Color::BLACK,
                            ..default()
                        },
                    )
                })),
        )
}

#[derive(Resource)]
struct HealthOptionMutable(Mutable<Option<Mutable<u32>>>);

#[derive(Event, Default)]
struct SpawnPlayer;

fn spawn_player(
    mut commands: Commands,
    health_option: Res<HealthOptionMutable>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let health = Mutable::new(PLAYER_HEALTH);
    commands.spawn((
        Player,
        Health(PLAYER_HEALTH),
        HealthMutable(health.clone()),
        PbrBundle {
            mesh: meshes.add(Mesh::from(Sphere {
                radius: RADIUS,
                ..default()
            })),
            transform: Transform::from_translation(PLAYER_POSITION),
            material: materials.add(Color::rgb_u8(228, 147, 58)),
            ..default()
        },
    ));
    health_option.0.set(Some(health));
}

fn respawn_button() -> impl Element {
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
        .on_click(|| spawn(async_world().send_event(SpawnPlayer)).detach())
        .child(El::<TextBundle>::new().text(Text::from_section(
            "respawn",
            TextStyle {
                font_size: 60.,
                color: Color::WHITE,
                ..default()
            },
        )))
}

fn ui_root(world: &mut World) {
    let style_data = world.resource::<StyleDataResource>().0.clone();
    let health_option = world.resource::<HealthOptionMutable>().0.clone();
    let starting_distance = CAMERA_POSITION.distance(PLAYER_POSITION);
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .child_signal(
            health_option
                .signal_cloned()
                .map_option(
                    move |health| {
                        health
                            .signal()
                            .map(|health| health > 0)
                            .dedupe()
                            .map_bool(
                                clone!((style_data) move || {
                                    Stack::<NodeBundle>::new()
                                    .width(Val::Percent(100.))
                                    .height(Val::Percent(100.))
                                        .with_style(|style| style.padding.bottom = Val::Px(10.))
                                        .layer(
                                            Column::<NodeBundle>::new()
                                                .with_style(|style| style.row_gap = Val::Px(MINI.1 / 4.))
                                                .on_signal_with_style(
                                                    style_data.signal(),
                                                    |style, StyleData { left, top, .. }| {
                                                        style.left = Val::Px(left - MINI.0 / 2.);
                                                        style.top = Val::Px(top - 30. * 2. - MINI.1 * 1.5);
                                                    },
                                                )
                                                // .on_signal_with_transform(style_data.signal(), move |transform, StyleData { scale, .. }| {
                                                //     transform.scale = Vec3::splat(starting_distance / scale);
                                                // })
                                                .item(
                                                    El::<TextBundle>::new()
                                                    .with_style(|style| style.left = Val::Px(MINI.1 / 4.))
                                                        .text(
                                                            Text::from_section(
                                                                NAME,
                                                                TextStyle {
                                                                    font_size: MINI.1 * 3. / 4.,
                                                                    color: Color::WHITE,
                                                                    ..default()
                                                                },
                                                            )
                                                        ),
                                                )
                                                .item(
                                                    healthbar(
                                                        PLAYER_HEALTH,
                                                        health.signal(),
                                                        MINI.1,
                                                        colorgrad::CustomGradient::new()
                                                            .html_colors(&["purple", "yellow"])
                                                            .build()
                                                            .unwrap(),
                                                    )
                                                    .width(Val::Px(MINI.0))
                                                    .height(Val::Px(MINI.1))
                                                ),
                                        )
                                        .layer(
                                            healthbar(
                                                PLAYER_HEALTH,
                                                health.signal(),
                                                MAXI.1,
                                                colorgrad::CustomGradient::new()
                                                    .html_colors(&["red", "green"])
                                                    .build()
                                                    .unwrap(),
                                            )
                                            .align(Align::new().bottom().center_x())
                                            .width(Val::Px(MAXI.0))
                                            .height(Val::Px(MAXI.1))
                                        )
                                        .type_erase()
                                }),
                                || respawn_button().type_erase(),
                            )
                            .boxed()
                    },
                    || always(respawn_button().type_erase()).boxed(),
                )
                .flatten(),
        )
        .spawn(world);
}

#[derive(Resource)]
struct HealthTickTimer(Timer);

fn decay(mut health: Query<&mut Health>, mut health_tick_timer: ResMut<HealthTickTimer>, time: Res<Time>) {
    if health_tick_timer.0.tick(time.delta()).finished() {
        let mut health = health.single_mut();
        health.0 = health.0.saturating_sub(1);
    }
}

fn despawn_when_dead(mut commands: Commands, query: Query<(Entity, &Health), Changed<Health>>) {
    if let Ok((entity, health)) = query.get_single() {
        if health.0 == 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}
