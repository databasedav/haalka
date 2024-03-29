// A UI on the right with a 3D scene of the character on the left.
//     The character can be simple 3D shapes.
// The UI is composed of multiple buttons to select options.
//     The selected option is highlighted.
//     There are too many buttons to fit in the box, so the box can be scrolled vertically. You can
//         duplicate buttons or choose a small box size to simulate this.
// Changing the selection in the UI changes the 3D shapes in the 3D scene.
// On the top of the UI is a text field for the character name.

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use haalka::*;
use strum::{self, IntoEnumIterator};

fn main() {
    let selected_shape = Mutable::new(Shape::Cube);
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
        .add_systems(Startup, (setup, ui_root).chain())
        .add_systems(Update, mouse_scroll)
        .insert_resource(SelectedShape(selected_shape))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

#[derive(Clone, Copy, PartialEq, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "lowercase")]
enum Shape {
    Cube,
    Box,
    Capsule,
    Torus,
    Cylinder,
    Sphere,
}

#[derive(Resource)]
struct SelectedShape(Mutable<Shape>);

fn button(shape: Shape, selected_shape: Mutable<Shape>) -> impl Element {
    let selected = selected_shape.signal().eq(shape);
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    let selected_hovered_broadcaster =
        map_ref!(selected, pressed_signal, hovered_signal => (*selected || *pressed_signal, *hovered_signal))
            .broadcast();
    let border_color_signal = {
        selected_hovered_broadcaster
            .signal()
            .map(|(selected, hovered)| {
                if selected {
                    Color::RED
                } else if hovered {
                    Color::WHITE
                } else {
                    Color::BLACK
                }
            })
            .map(BorderColor)
    };
    let background_color_signal = {
        selected_hovered_broadcaster
            .signal()
            .map(|(selected, hovered)| {
                if selected {
                    CLICKED_BUTTON
                } else if hovered {
                    HOVERED_BUTTON
                } else {
                    NORMAL_BUTTON
                }
            })
            .map(BackgroundColor)
    };
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Px(250.);
            style.height = Val::Px(65.);
            style.border = UiRect::all(Val::Px(5.));
        })
        .align_content(Align::center())
        .border_color_signal(border_color_signal)
        .background_color_signal(background_color_signal)
        .hovered_sync(hovered)
        .pressed_sync(pressed)
        .on_click(move || selected_shape.set_neq(shape))
        .child(El::<TextBundle>::new().text(Text::from_section(
            shape.to_string(),
            TextStyle {
                font_size: 40.0,
                color: Color::rgb(0.9, 0.9, 0.9),
                ..default()
            },
        )))
}

fn ui_root(world: &mut World) {
    let selected_shape = world.resource::<SelectedShape>().0.clone();
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .child(
            Stack::<NodeBundle>::new()
                .with_style(|style| {
                    style.width = Val::Percent(100.);
                    style.height = Val::Percent(100.);
                })
                .layer(
                    El::<NodeBundle>::new()
                        .align(Align::new().center_y().right())
                        .with_style(|style| {
                            style.padding.right = Val::Percent(20.);
                            style.height = Val::Px(200.);
                            style.overflow = Overflow::clip_y();
                        })
                        .child({
                            let position = Mutable::new(0.);
                            Column::<NodeBundle>::new()
                                .on_signal_with_style(position.signal(), |style, position| {
                                    style.top = Val::Px(position);
                                })
                                .update_raw_el(|raw_el| raw_el.insert(Scrollable { position }))
                                .items(Shape::iter().map(move |shape| button(shape, selected_shape.clone())))
                        }),
                ),
        )
        .spawn(world);
}

#[derive(Component, Default)]
struct Scrollable {
    position: Mutable<f32>,
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&Scrollable, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (scrollable, parent, list_node) in &mut query_list {
            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;
            let max_scroll = (items_height - container_height).max(0.);
            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };
            let mut position = scrollable.position.lock_mut();
            *position = (*position + dy).clamp(-max_scroll, 0.);
        }
    }
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>, selected_shape: Res<SelectedShape>) {
    spawn(selected_shape.0.signal().for_each(|shape| {
        async_world().apply(move |world: &mut World| {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            *world.query::<&mut Handle<Mesh>>().single_mut(world) = meshes.add(match shape {
                Shape::Cube => shape::Cube::default().into(),
                Shape::Box => shape::Box::default().into(),
                Shape::Capsule => shape::Capsule::default().into(),
                Shape::Torus => shape::Torus::default().into(),
                Shape::Cylinder => shape::Cylinder::default().into(),
                Shape::Sphere => shape::Icosphere::default().try_into().unwrap(),
            });
        })
    }))
    .detach();
    commands.spawn(PbrBundle {
        material: materials.add(Color::rgb_u8(87, 108, 50).into()),
        transform: Transform::from_xyz(-5., 1., 0.).looking_at(Vec3::new(4.0, 0., 12.0), Vec3::Y),
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0., 8., 0.),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(-5., 1., 0.), Vec3::Y),
        ..default()
    });
}
