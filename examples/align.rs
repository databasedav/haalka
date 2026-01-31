//! Alignment API demo, port of <https://github.com/MoonZoon/MoonZoon/tree/main/examples/align> and <https://github.com/MoonZoon/MoonZoon/tree/main/examples/align_content>.

mod utils;
use utils::*;

use std::ops::{Deref, DerefMut};

use bevy::{color::palettes, prelude::*};
use haalka::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
                },
                camera,
            ),
        )
        .insert_resource(SelfAlignmentEnabled(true))
        .insert_resource(ContentAlignmentEnabled(false))
        .insert_resource(StripeDirection(haalka::stripe::Direction::Row))
        .insert_resource(RectangleSelfAlignment::default())
        .insert_resource(RectangleContentAlignment::default())
        .run();
}

#[derive(Clone, Copy, EnumIter, Display, PartialEq)]
#[strum(crate = "strum")]
enum RectangleAlignment {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Center,
}

impl RectangleAlignment {
    fn to_align(self) -> Align {
        match self {
            Self::TopLeft => Align::new().top().left(),
            Self::Top => Align::new().top().center_x(),
            Self::TopRight => Align::new().top().right(),
            Self::Right => Align::new().right().center_y(),
            Self::BottomRight => Align::new().bottom().right(),
            Self::Bottom => Align::new().bottom().center_x(),
            Self::BottomLeft => Align::new().bottom().left(),
            Self::Left => Align::new().left().center_y(),
            Self::Center => Align::center(),
        }
    }
}

#[derive(Clone, Copy, Resource, Deref, DerefMut)]
struct SelfAlignmentEnabled(bool);

#[derive(Clone, Copy, Resource, Deref, DerefMut)]
struct ContentAlignmentEnabled(bool);

#[derive(Clone, Copy, Resource, Deref, DerefMut)]
struct StripeDirection(haalka::stripe::Direction);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct RectangleSelfAlignment(Option<RectangleAlignment>);

#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
struct RectangleContentAlignment(Option<RectangleAlignment>);

fn alignment_button<R: Resource + Clone + Deref<Target = bool> + DerefMut>(label: &str) -> impl Element {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert((Pickable::default(), Hoverable))
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_node(|mut node| {
            node.width = Val::Px(250.);
            node.height = Val::Px(80.);
        })
        .lazy_entity(lazy_entity.clone())
        .background_color_signal({
            let hovered = signal::from_entity(lazy_entity).has_component::<Hovered>().dedupe();
            let enabled = signal::from_resource_changed::<R>().map_in(deref_copied);

            signal::any!(hovered, enabled)
                .dedupe()
                .map_bool_in(|| palettes::basic::GRAY.into(), || Color::BLACK)
                .map_in(BackgroundColor)
                .map_in(Some)
        })
        .align_content(Align::center())
        .on_click(|_: In<_>, mut enabled: ResMut<R>| {
            **enabled = !**enabled;
        })
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(label)),
        )
}

fn ui_root() -> impl Element {
    Column::<Node>::new()
        .ui_root()
        .insert(Pickable::default())
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
            node.row_gap = Val::Px(20.);
            node.padding = UiRect::all(Val::Px(20.));
        })
        .align_content(Align::center())
        .cursor(CursorIcon::default())
        .item(
            Column::<Node>::new()
                .with_node(|mut node| node.row_gap = Val::Px(15.))
                .item(
                    Row::<Node>::new()
                        .with_node(|mut node| node.column_gap = Val::Px(15.))
                        .item(container(simple_header("El"), El::<Node>::new().child(rectangle(1))))
                        .item(container(
                            simple_header("Column"),
                            Column::<Node>::new().items(rectangles()),
                        ))
                        .item(container(simple_header("Row"), Row::<Node>::new().items(rectangles()))),
                )
                .item(
                    Row::<Node>::new()
                        .with_node(|mut node| node.column_gap = Val::Px(15.))
                        .item(container(
                            stripe_header(),
                            Stripe::<Node>::new()
                                .direction_signal(
                                    signal::from_resource_changed::<StripeDirection>().map_in(deref_copied),
                                )
                                .items(rectangles()),
                        ))
                        .item(container(
                            simple_header("Grid"),
                            Grid::<Node>::new().cells(rectangles()),
                        ))
                        .item(container(
                            simple_header("Stack"),
                            Stack::<Node>::new().layers(rectangles()),
                        )),
                ),
        )
        .item(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(30.))
                .align(Align::new().center_x())
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(10.))
                        .item(alignment_button::<SelfAlignmentEnabled>("align self").align(Align::center()))
                        .item(
                            Stack::<Node>::new()
                                .layers(RectangleAlignment::iter().map(alignment_switcher::<RectangleSelfAlignment>))
                                .visibility_signal(
                                    signal::from_resource_changed::<SelfAlignmentEnabled>()
                                        .map_in(deref_copied)
                                        .map_bool_in(|| Visibility::Inherited, || Visibility::Hidden)
                                        .map_in(Some),
                                )
                                .apply(switcher_container_node),
                        ),
                )
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(10.))
                        .item(alignment_button::<ContentAlignmentEnabled>("align content").align(Align::center()))
                        .item(
                            Stack::<Node>::new()
                                .layers(RectangleAlignment::iter().map(alignment_switcher::<RectangleContentAlignment>))
                                .visibility_signal(
                                    signal::from_resource_changed::<ContentAlignmentEnabled>()
                                        .map_in(deref_copied)
                                        .map_bool_in(|| Visibility::Inherited, || Visibility::Hidden)
                                        .map_in(Some),
                                )
                                .apply(switcher_container_node),
                        ),
                ),
        )
}

fn container_node<E: Element>(el: E) -> E {
    el.with_builder(|builder| {
        builder
            .insert(BorderColor::all(palettes::basic::GRAY))
            .with_component::<Node>(|mut node| {
                node.height = Val::Px(220.);
                node.width = Val::Px(260.);
                node.border = UiRect::all(Val::Px(3.));
            })
    })
}

fn switcher_container_node<E: Element>(el: E) -> E {
    el.with_builder(|builder| {
        builder
            .insert(BorderColor::all(palettes::basic::GRAY))
            .with_component::<Node>(|mut node| {
                node.height = Val::Px(180.);
                node.width = Val::Px(260.);
                node.border = UiRect::all(Val::Px(2.));
            })
    })
}

fn container(header: impl Element, element: impl Element) -> impl Element {
    Column::<Node>::new().item(header.align(Align::new().center_x())).item(
        element
            .align_content_signal(
                signal::from_resource_changed::<ContentAlignmentEnabled>()
                    .map_in(deref_copied)
                    .map_true_in(|| {
                        signal::from_resource_changed::<RectangleContentAlignment>()
                            .map_in(deref_copied)
                            .map_some_in(RectangleAlignment::to_align)
                    })
                    .map_in(signal::option)
                    .flatten()
                    .map_in(Option::flatten)
                    .dedupe(),
            )
            .apply(container_node),
    )
}

fn simple_header(name: &str) -> impl Element {
    El::<Text>::new()
        .text_font(TextFont::from_font_size(32.))
        .text(Text::new(name))
}

fn stripe_header() -> impl Element {
    Row::<Node>::new()
        .align_content(Align::center())
        .with_node(|mut node| {
            node.column_gap = Val::Px(10.);
            node.height = Val::Px(40.);
        })
        .item(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(32.))
                .text(Text::new("Stripe")),
        )
        .item(
            Column::<Node>::new()
                .align_content(Align::center())
                .with_node(|mut node| {
                    node.row_gap = Val::Px(4.);
                })
                .item(stripe_direction_button("row", haalka::stripe::Direction::Row))
                .item(stripe_direction_button("col", haalka::stripe::Direction::Column)),
        )
}

fn stripe_direction_button(label: &str, direction: haalka::stripe::Direction) -> impl Element {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert((Pickable::default(), Hoverable))
        .align_content(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_node(|mut node| {
            node.width = Val::Px(55.);
            node.height = Val::Px(20.);
        })
        .lazy_entity(lazy_entity.clone())
        .background_color_signal({
            let selected = signal::from_resource_changed::<StripeDirection>()
                .map_in(deref_copied)
                .eq(direction);
            let hovered = signal::from_entity(lazy_entity).has_component::<Hovered>();

            signal::any!(selected, hovered)
                .dedupe()
                .map_bool_in(|| palettes::css::MIDNIGHT_BLUE, || palettes::basic::BLUE)
                .map_in(BackgroundColor::from)
                .map_in(Some)
        })
        .on_click(move |_: In<_>, mut stripe_direction: ResMut<StripeDirection>| {
            **stripe_direction = direction;
        })
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(12.))
                .text(Text::new(label)),
        )
}

fn rectangle(index: i32) -> impl Element {
    let size = 55;
    El::<Node>::new()
        .with_node(move |mut node| {
            node.width = Val::Px(size as f32);
            node.height = Val::Px(size as f32)
        })
        .background_color(BackgroundColor(palettes::css::DARK_GREEN.into()))
        .align_signal(
            signal::from_resource_changed::<SelfAlignmentEnabled>()
                .map_in(deref_copied)
                .map_true_in(|| {
                    signal::from_resource_changed::<RectangleSelfAlignment>()
                        .map_in(deref_copied)
                        .map_some_in(RectangleAlignment::to_align)
                })
                .map_in(signal::option)
                .flatten()
                .map_in(Option::flatten)
                .dedupe(),
        )
        .child(
            El::<Text>::new()
                .align(Align::center())
                .text_font(TextFont::from_font_size(18.))
                .text(Text(index.to_string())),
        )
}

fn rectangles() -> Vec<impl Element> {
    (1..=2).map(rectangle).collect()
}

fn alignment_switcher<R>(rectangle_alignment: RectangleAlignment) -> impl Element
where
    R: Resource + Clone + Copy + Deref<Target = Option<RectangleAlignment>> + DerefMut,
{
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert((Pickable::default(), Hoverable))
        .align(rectangle_alignment.to_align())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal({
            let selected = signal::from_resource_changed::<R>()
                .map_in(deref_copied)
                .eq(Some(rectangle_alignment));
            let hovered = signal::from_entity(lazy_entity).has_component::<Hovered>();

            signal::any!(selected, hovered)
                .dedupe()
                .map_bool_in(|| palettes::css::MIDNIGHT_BLUE, || palettes::basic::BLUE)
                .map_in(BackgroundColor::from)
                .map_in(Some)
        })
        .with_node(|mut node| node.padding = UiRect::all(Val::Px(4.)))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(11.))
                .text(Text(rectangle_alignment.to_string())),
        )
        .on_click(move |_: In<_>, mut alignment: ResMut<R>| {
            if **alignment == Some(rectangle_alignment) {
                **alignment = None;
            } else {
                **alignment = Some(rectangle_alignment);
            }
        })
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
