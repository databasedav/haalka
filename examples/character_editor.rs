//! - A UI on the right with a 3D scene of the character on the left.
//!   - The character can be simple 3D shapes.
//! - The UI is composed of multiple buttons to select options.
//!   - The selected option is highlighted.
//!   - There are too many buttons to fit in the box, so the box can be scrolled vertically. You can
//!     duplicate buttons or choose a small box size to simulate this.
//! - Changing the selection in the UI changes the 3D shapes in the 3D scene.
//! - On the top of the UI is a text field for the character name.

mod utils;
use utils::*;

use bevy::{input_focus::InputFocus, prelude::*, ui::Pressed};
use bevy_ui_text_input::{TextInputMode, TextInputPrompt};
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

#[derive(Clone, Copy, PartialEq, strum::Display, strum::EnumIter, strum::AsRefStr)]
#[strum(serialize_all = "lowercase")]
enum Shape {
    Sphere,
    Plane,
    Cuboid,
    Cylinder,
    Capsule3d,
    Torus,
}

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct SelectedShape(Shape);

fn button(shape: Shape) -> impl Element {
    let lazy_entity = LazyEntity::new();

    let selected = signal::from_resource_changed::<SelectedShape>()
        .map_in(deref_copied)
        .eq(shape);
    let pressed = signal::from_entity(lazy_entity.clone())
        .has_component::<Pressed>()
        .dedupe();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();

    let selected_hovered = signal::zip!(signal::any!(selected, pressed), hovered).dedupe();

    El::<Node>::new()
        .insert((Pickable::default(), Hoverable, Pressable))
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_node(|mut node| {
            node.width = Val::Px(BUTTON_WIDTH);
            node.height = Val::Px(BUTTON_HEIGHT);
            node.border = UiRect::all(Val::Px(5.));
        })
        .align_content(Align::center())
        .lazy_entity(lazy_entity.clone())
        .border_color_signal(
            selected_hovered
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
            selected_hovered
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

fn name_input(shape_buttons: LazyEntity) -> impl Element {
    let text_input = LazyEntity::new();
    El::<Node>::new()
        .insert((BackgroundColor(NORMAL_BUTTON), Pickable::default()))
        .cursor(CursorIcon::System(SystemCursorIcon::Text))
        .with_node(|mut node| node.height = Val::Px(BUTTON_HEIGHT))
        .on_click(
            clone!((text_input) move |In(_), mut input_focus: ResMut<InputFocus>| input_focus.0 = Some(*text_input)),
        )
        .on_click_outside(|In(_), mut input_focus: ResMut<InputFocus>| input_focus.0 = None)
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
                .text_color(TextColor(Color::WHITE))
                .text_input_prompt(TextInputPrompt {
                    text: "name".to_string(),
                    color: Some(bevy::color::palettes::basic::GRAY.into()),
                    ..default()
                })
                .lazy_entity(text_input.clone())
                .on_change(clone!((shape_buttons) move |In((_, text)),
                    mut scroll_positions: Query<&mut ScrollPosition>,
                    mut commands: Commands| {
                    if let Some((i, shape)) = Shape::iter()
                        .enumerate()
                        .find(|(_, shape)| shape.as_ref() == text)
                    {
                        commands.trigger(SetShape(shape));
                        scroll_positions.get_mut(*shape_buttons).unwrap().y = i as f32 * BUTTON_HEIGHT;
                    }
                })),
        )
}

fn shape_buttons_list(shape_buttons: LazyEntity) -> impl Element {
    Column::<Node>::new()
        .lazy_entity(shape_buttons)
        .with_node(|mut node| node.height = Val::Px(200.))
        .align(Align::new().center_x())
        .insert(Pickable::default())
        .mutable_viewport(Overflow::scroll_y())
        .on_scroll_on_hover(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(20.)
                .into_system(),
        )
        .items(Shape::iter().map(button))
}

fn editor_panel() -> impl Element {
    let shape_buttons = LazyEntity::new();
    Column::<Node>::new()
        .align(Align::new().center_y().right())
        .with_node(|mut node| {
            node.padding.right = Val::Percent(20.);
            node.row_gap = Val::Px(20.);
        })
        .item(name_input(shape_buttons.clone()))
        .item(shape_buttons_list(shape_buttons))
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .ui_root()
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .child(editor_panel())
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
