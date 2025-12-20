//! - A UI on the right with a 3D scene of the character on the left.
//!   - The character can be simple 3D shapes.
//! - The UI is composed of multiple buttons to select options.
//!   - The selected option is highlighted.
//!   - There are too many buttons to fit in the box, so the box can be scrolled vertically. You can
//!     duplicate buttons or choose a small box size to simulate this.
//! - Changing the selection in the UI changes the 3D shapes in the 3D scene.
//! - On the top of the UI is a text field for the character name.

mod utils;
use bevy_input_focus::InputFocus;
use bevy_ui::Pressed;
use bevy_ui_text_input::{TextInputMode, TextInputPrompt};
use utils::*;

use bevy::prelude::*;
use haalka::prelude::*;
use strum::{self, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(
            Startup,
            (
                (setup, |world: &mut World| {
                    ui_root().spawn(world);
                })
                    .chain(),
                |mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands| {
                    commands.spawn((
                        MeshMaterial3d(materials.add(Color::srgb_u8(87, 108, 50))),
                        Transform::from_xyz(-1., 0., 1.),
                    ));
                    commands.trigger(SetShape(Shape::Sphere));
                },
            ),
        )
        .insert_resource(SelectedShape(Shape::Cuboid))
        .insert_resource(ScrollPosition::default())
        .add_observer(
            |event: On<SetShape>,
             character: Single<Entity, With<MeshMaterial3d<StandardMaterial>>>,
             mut meshes: ResMut<Assets<Mesh>>,
             mut selected_shape: ResMut<SelectedShape>,
             mut commands: Commands| {
                let shape = **event;
                if let Ok(mut entity) = commands.get_entity(*character) {
                    entity.insert(Mesh3d(meshes.add(match shape {
                        Shape::Sphere => Sphere::default().mesh().ico(5).unwrap(),
                        Shape::Plane => Plane3d::default().mesh().size(1., 1.).into(),
                        Shape::Cuboid => Cuboid::default().into(),
                        Shape::Cylinder => Cylinder::default().into(),
                        Shape::Capsule3d => Capsule3d::default().into(),
                        Shape::Torus => Torus::default().into(),
                    })));
                }
                **selected_shape = shape;
            },
        )
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const BUTTON_WIDTH: f32 = 250.;
const BUTTON_HEIGHT: f32 = 50.;

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

#[derive(Resource, Clone, Copy, PartialEq, Deref, DerefMut)]
struct SelectedShape(Shape);

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct ScrollPosition(f32);

fn button(shape: Shape) -> impl Element {
    let lazy_entity = LazyEntity::new();

    let selected_pressed_hovered_signal = SignalBuilder::from_resource::<SelectedShape>()
        .map_in(deref_copied)
        .dedupe()
        .eq(shape)
        .combine(
            SignalBuilder::from_lazy_entity(lazy_entity.clone())
                .map(|In(entity), presseds: Query<&Pressed>| presseds.contains(entity))
                .dedupe(),
        )
        .combine(
            SignalBuilder::from_lazy_entity(lazy_entity.clone())
                .has_component::<Hovered>()
                .dedupe(),
        )
        .map_in(|((selected, pressed), hovered)| (selected || pressed, hovered))
        .dedupe();

    El::<Node>::new()
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_node(|mut node| {
            node.width = Val::Px(BUTTON_WIDTH);
            node.height = Val::Px(BUTTON_HEIGHT);
            node.border = UiRect::all(Val::Px(5.));
        })
        .align_content(Align::center())
        .lazy_entity(lazy_entity.clone())
        .border_color_signal(
            selected_pressed_hovered_signal
                .clone()
                .map_in(|(selected, hovered)| {
                    if selected {
                        bevy::color::palettes::basic::RED.into()
                    } else if hovered {
                        Color::WHITE
                    } else {
                        Color::BLACK
                    }
                })
                .map_in(BorderColor::all)
                .map_in(Some),
        )
        .background_color_signal(
            selected_pressed_hovered_signal
                .map_in(|(selected, hovered)| {
                    if selected {
                        CLICKED_BUTTON
                    } else if hovered {
                        HOVERED_BUTTON
                    } else {
                        NORMAL_BUTTON
                    }
                })
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .on_click(move |_: In<_>, mut commands: Commands| {
            commands.trigger(SetShape(shape));
        })
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(33.33))
                .text_color(TextColor(Color::srgb(0.9, 0.9, 0.9)))
                .text(Text(shape.to_string())),
        )
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .ui_root()
        .cursor(CursorIcon::default())
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .child(
            Stack::<Node>::new()
                .with_node(|mut node| {
                    node.width = Val::Percent(100.);
                    node.height = Val::Percent(100.);
                })
                .layer(
                    Column::<Node>::new()
                        .align(Align::new().center_y().right())
                        .with_node(|mut node| {
                            node.padding.right = Val::Percent(20.);
                            node.row_gap = Val::Px(20.);
                        })
                        .item({
                            let lazy_entity = LazyEntity::new();
                            El::<Node>::new()
                                .insert(BackgroundColor(NORMAL_BUTTON))
                                .with_node(|mut node| node.height = Val::Px(BUTTON_HEIGHT))
                                .child(
                                    TextInput::new()
                                        .with_node(|mut node| {
                                            node.left = Val::Px(10.);
                                            node.height = Val::Px(BUTTON_HEIGHT - 10. * 2.);
                                            node.width = Val::Px(BUTTON_WIDTH - 10.);
                                        })
                                        .align(Align::new().center_y())
                                        .with_text_input_node(|mut node| {
                                            node.mode = TextInputMode::SingleLine;
                                            // TODO: https://github.com/ickshonpe/bevy_ui_text_input/issues/10
                                            // node.justification = Justify::Center;
                                        })
                                        .cursor(CursorIcon::System(SystemCursorIcon::Text))
                                        .text_color(TextColor(Color::WHITE))
                                        .text_input_prompt(TextInputPrompt {
                                            text: "name".to_string(),
                                            color: Some(bevy::color::palettes::basic::GRAY.into()),
                                            ..default()
                                        })
                                        .lazy_entity(lazy_entity.clone())
                                        .on_change_with_system(
                                            |In((_, text)),
                                             mut scroll_pos: ResMut<ScrollPosition>,
                                             mut commands: Commands| {
                                                if let Some((i, shape)) = Shape::iter()
                                                    .enumerate()
                                                    .find(|(_, shape)| shape.to_string() == text)
                                                {
                                                    commands.trigger(SetShape(shape));
                                                    **scroll_pos = i as f32 * BUTTON_HEIGHT;
                                                }
                                            },
                                        )
                                        .on_click_outside(|In(_), mut commands: Commands| {
                                            commands.insert_resource(InputFocus(None))
                                        }),
                                )
                        })
                        .item({
                            Column::<Node>::new()
                                .with_node(|mut node| node.height = Val::Px(200.))
                                .align(Align::new().center_x())
                                .mutable_viewport(haalka::prelude::Axis::Vertical)
                                .on_scroll_with_system_on_hover(
                                    BasicScrollHandler::new()
                                        .direction(ScrollDirection::Vertical)
                                        .pixels(20.)
                                        .into_system(),
                                )
                                .viewport_y_signal(
                                    SignalBuilder::from_resource::<ScrollPosition>().map_in(deref_copied),
                                )
                                .items(Shape::iter().map(button))
                        }),
                ),
        )
}

#[derive(Event, Deref)]
struct SetShape(Shape);

fn setup(mut commands: Commands) {
    commands.spawn((
        PointLight {
            intensity: 1_500_000.,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0., 8., 0.),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3., 3., 3.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
    ));
}
