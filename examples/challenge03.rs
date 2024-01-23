// Simple 3D Scene with a character (sphere).
//     Can be moved around with WASD/arrow keys.
// A health bar and character name is anchored to the character in world-space.
// The health starts at 10 and decreases by 1 every second. The health should be stored and managed
// in Bevy ECS. When reaching 0 HP, the character should be despawned together with UI.

use bevy::prelude::*;
use futures_signals::signal::Mutable;
use futures_signals_ext::*;
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
        .add_systems(Startup, (setup, ui_root, wasd))
        .add_systems(Update, wasd)
        .insert_resource(XZ(Mutable::new((0., 0.))))
        .run();
}

struct PointLightEl<Bundle = PointLightBundle>(RawHaalkaEl<Bundle>);

impl PointLightEl {
    pub fn new() -> Self {
        Self(RawHaalkaEl::from(PointLightBundle::default()))
    }
}

impl RawElWrapper for PointLightEl {
    type NodeType = PointLightBundle;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<Self::NodeType> {
        &mut self.0
    }
}

impl_node_methods! {
    PointLightEl => {
        PointLightBundle => [
            point_light: PointLight,
            transform: Transform,
        ],
    },
}

#[derive(Resource)]
struct XZ(Mutable<(f32, f32)>);

fn setup(world: &mut World) {
    let xz = world.resource::<XZ>().0.clone();
    RawHaalkaEl::from(PbrBundle {
        mesh: world
            .resource_mut::<Assets<Mesh>>()
            .add(shape::Plane::from_size(50.0).into()),
        material: world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(Color::WHITE.into()),
        ..default()
    })
    .child(
        // instantly add haalka's reactivity to *any* `impl Bundle`
        RawHaalkaEl::from(PbrBundle {
            mesh: world.resource_mut::<Assets<Mesh>>().add(Mesh::from(shape::UVSphere {
                radius: 0.5,
                ..default()
            })),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .on_signal_with_component::<_, Transform>(xz.signal(), |transform, (x, z)| {
            (transform.translation.x, transform.translation.z) = (x, z)
        })
        .component_one_shot_signal(
            xz.signal(),
            |In((_entity, (x, _))): In<(Entity, (f32, f32))>, mut materials: ResMut<Assets<StandardMaterial>>| {
                let x = (x * 5.).round() as u8;
                materials.add(
                    Color::rgb_u8(
                        128u8.saturating_add(x),
                        128u8.saturating_add(x),
                        128u8.saturating_add(x),
                    )
                    .into(),
                )
            },
        ),
    )
    .spawn(world);
    // or use the `impl_node_methods!` macro and a tiny bit of boilerplate (`impl RawElWrapper for
    // PointLightEl`) to expose high level haalka-esque convenience methods for reactively
    // manipulating particular components
    PointLightEl::new()
        .point_light(PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        })
        .transform(Transform::from_xyz(0., 8., 0.))
        .spawn(world);
    RawHaalkaEl::from(Camera3dBundle {
        transform: Transform::from_xyz(0., 8.5, 8.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    })
    .spawn(world);
}

fn wasd(keys: Res<Input<KeyCode>>, xz: Res<XZ>) {
    if keys.pressed(KeyCode::A) {
        xz.0.update_mut(|(x, _)| *x -= 0.1);
    } else if keys.pressed(KeyCode::D) {
        xz.0.update_mut(|(x, _)| *x += 0.1);
    } else if keys.pressed(KeyCode::W) {
        xz.0.update_mut(|(_, z)| *z -= 0.1);
    } else if keys.pressed(KeyCode::S) {
        xz.0.update_mut(|(_, z)| *z += 0.1);
    }
}

fn ui_root(world: &mut World) {
    let xz = world.resource::<XZ>().0.clone();
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
        })
        .child(
            El::<NodeBundle>::new()
                .with_style(|style| {
                    style.width = Val::Px(100.);
                    style.height = Val::Px(50.);
                })
                .align(Align::center())
                .background_color(Color::BLACK.into())
                .on_signal_with_style(xz.signal(), |style, (x, z)| {
                    style.left = Val::Px(x * 50.);
                    style.top = Val::Px(z * 50.);
                }),
        )
        .spawn(world);
}
