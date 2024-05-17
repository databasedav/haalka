// A UI on the right with a 3D scene of the character on the left.
//     The character can be simple 3D shapes.
// The UI is composed of multiple buttons to select options.
//     The selected option is highlighted.
//     There are too many buttons to fit in the box, so the box can be scrolled vertically. You can
//         duplicate buttons or choose a small box size to simulate this.
// Changing the selection in the UI changes the 3D shapes in the 3D scene.
// On the top of the UI is a text field for the character name.

use std::convert::identity;

use bevy::prelude::*;
use haalka::*;
use strum::{self, IntoEnumIterator};

fn main() {
    let selected_shape = Mutable::new(Shape::Cuboid);
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
        .insert_resource(SelectedShape(selected_shape))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
const BUTTON_HEIGHT: Val = Val::Px(65.);

#[derive(Clone, Copy, PartialEq, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "lowercase")]
enum Shape {
    Sphere,
    Plane,
    Cuboid,
    Cylinder,
    Capsule3d,
    Torus,
}

#[derive(Resource)]
struct SelectedShape(Mutable<Shape>);

fn button(shape: Shape, selected_shape: Mutable<Shape>, hovered: Mutable<bool>) -> impl Element {
    let selected = selected_shape.signal().eq(shape);
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let hovered_signal = hovered.signal();
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
            style.height = BUTTON_HEIGHT;
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
                    Column::<NodeBundle>::new()
                        .align(Align::new().center_y().right())
                        .with_style(|style| {
                            style.padding.right = Val::Percent(20.);
                            style.row_gap = Val::Px(20.);
                        })
                        .item(
                            El::<NodeBundle>::new()
                                .align(Align::new().top().center_x())
                                .with_style(|style| {
                                    style.padding.right = Val::Percent(20.);
                                })
                                .child(El::<TextBundle>::new().text(Text::from_section(
                                    "character name",
                                    TextStyle {
                                        font_size: 40.0,
                                        color: Color::WHITE,
                                        ..default()
                                    },
                                ))),
                        )
                        .item({
                            let hovereds = MutableVec::new_with_values(
                                (0..Shape::iter().count()).map(|_| Mutable::new(false)).collect(),
                            );
                            Column::<NodeBundle>::new()
                                .height(Val::Px(200.))
                                .align(Align::new().center_x())
                                // TODO: hovering must be manually managed when children have their own hover handlers until mouseenter/mouseleave events in mod picking https://discord.com/channels/691052431525675048/1038322714320052304/1240468289512276000
                                // .scrollable_on_hover(...)
                                .scrollable(
                                    ScrollabilitySettings {
                                        flex_direction: FlexDirection::Column,
                                        overflow: Overflow::clip_y(),
                                        scroll_handler: BasicScrollHandler::new()
                                            .direction(ScrollDirection::Vertical)
                                            .pixels(20.)
                                            .into(),
                                    },
                                    hovereds
                                        .signal_vec_cloned()
                                        .map_signal(|hovered| hovered.signal())
                                        .to_signal_map(|hovereds| hovereds.iter().copied().any(identity))
                                        .dedupe(),
                                )
                                .items({
                                    let hovereds = hovereds.lock_ref().into_iter().cloned().collect::<Vec<_>>();
                                    Shape::iter()
                                        .zip(hovereds)
                                        .map(move |(shape, hovered)| button(shape, selected_shape.clone(), hovered))
                                })
                        }),
                ),
        )
        .spawn(world);
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>, selected_shape: Res<SelectedShape>) {
    spawn(selected_shape.0.signal().for_each(|shape| {
        async_world().apply(move |world: &mut World| {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            *world.query::<&mut Handle<Mesh>>().single_mut(world) = meshes.add(match shape {
                Shape::Sphere => Sphere::default().mesh().ico(5).unwrap(),
                Shape::Plane => Plane3d::default().mesh().size(1., 1.).into(),
                Shape::Cuboid => Cuboid::default().into(),
                Shape::Cylinder => Cylinder::default().into(),
                Shape::Capsule3d => Capsule3d::default().into(),
                Shape::Torus => Torus::default().into(),
            });
        })
    }))
    .detach();
    commands.spawn(PbrBundle {
        material: materials.add(Color::rgb_u8(87, 108, 50)),
        transform: Transform::from_xyz(-1., 0., 1.),
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1_500_000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0., 8., 0.),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3., 3., 3.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        ..default()
    });
}
