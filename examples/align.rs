//! Alignment API demo, port of <https://github.com/MoonZoon/MoonZoon/tree/main/examples/align> and <https://github.com/MoonZoon/MoonZoon/tree/main/examples/align_content>.

mod utils;
use utils::*;

use bevy::prelude::*;
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
        .insert_resource(Alignment::Self_)
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

#[derive(Clone, Copy, PartialEq, Resource)]
enum Alignment {
    Self_,
    Content,
}

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct RectangleSelfAlignment(Option<RectangleAlignment>);

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct RectangleContentAlignment(Option<RectangleAlignment>);

fn alignment_button(alignment: Alignment) -> impl Element {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .align(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_node(|mut node| {
            node.width = Val::Px(250.);
            node.height = Val::Px(80.);
        })
        .with_builder(|builder| builder.lazy_entity(lazy_entity.clone()))
        .background_color_signal(
            signal::or!(
                SignalBuilder::from_lazy_entity(lazy_entity)
                    .has_component::<Hovered>()
                    .dedupe(),
                SignalBuilder::from_resource::<Alignment>().dedupe().eq(alignment)
            )
            .dedupe()
            .map_bool_in(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
            .map_in(BackgroundColor)
            .map_in(Some),
        )
        .align_content(Align::center())
        .on_click(move |_: In<_>, mut current_alignment: ResMut<Alignment>| {
            *current_alignment = alignment;
        })
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(match alignment {
                    Alignment::Self_ => "align self",
                    Alignment::Content => "align content",
                })),
        )
}

fn ui_root() -> impl Element {
    Column::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
            node.row_gap = Val::Px(15.);
        })
        .align(Align::center())
        .align_content(Align::center())
        .cursor(CursorIcon::default())
        .item(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.))
                .item(container("Column", Column::<Node>::new().items(rectangles())))
                .item(container("El", El::<Node>::new().child(rectangle(1))))
                .item(container("Grid", Grid::<Node>::new().cells(rectangles()))),
        )
        .item(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.))
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(15.))
                        .item(alignment_button(Alignment::Self_))
                        .item(alignment_button(Alignment::Content)),
                )
                .item(
                    Stack::<Node>::new()
                        .layers(RectangleAlignment::iter().map(align_switcher))
                        .apply(container_node),
                ),
        )
        .item(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.))
                .item(container("Row", Row::<Node>::new().items(rectangles())))
                .item(container("Stack", Stack::<Node>::new().layers(rectangles()))),
        )
}

fn container_node<E: Element>(el: E) -> E {
    el.with_builder(|builder| {
        builder
            .insert(BorderColor::all(bevy::color::palettes::basic::GRAY))
            .with_component::<Node>(|mut node| {
                node.height = Val::Px(200.);
                node.width = Val::Px(278.);
                node.border = UiRect::all(Val::Px(3.));
            })
    })
}

fn container(name: &str, element: impl Element) -> impl Element {
    Column::<Node>::new()
        .item(
            El::<Text>::new()
                .align(Align::new().center_x())
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(name)),
        )
        .item(
            element
                .align_content_signal(
                    SignalBuilder::from_resource::<Alignment>()
                        .dedupe()
                        .eq(Alignment::Content)
                        .map_true_in(|| {
                            SignalBuilder::from_resource::<RectangleContentAlignment>()
                                .dedupe()
                                .map_in(deref_copied)
                                .map_in(RectangleAlignment::to_align)
                        })
                        .map_in(signal::option)
                        .flatten(),
                )
                .apply(container_node),
        )
}

fn rectangle(index: i32) -> impl Element {
    let size = 40;
    El::<Node>::new()
        .with_node(move |mut node| {
            node.width = Val::Px(size as f32);
            node.height = Val::Px(size as f32)
        })
        .background_color(BackgroundColor(bevy::color::palettes::css::DARK_GREEN.into()))
        .align_signal(
            SignalBuilder::from_resource::<Alignment>()
                .dedupe()
                .eq(Alignment::Self_)
                .map_true_in(|| {
                    SignalBuilder::from_resource::<RectangleSelfAlignment>()
                        .dedupe()
                        .map_in(deref_copied)
                        .map_some_in(RectangleAlignment::to_align)
                })
                .map_in(signal::option)
                .flatten()
                .map_in(Option::flatten),
        )
        .child(
            El::<Text>::new()
                .align(Align::center())
                .text_font(TextFont::from_font_size(11.67))
                .text(Text(index.to_string())),
        )
}

fn rectangles() -> Vec<impl Element> {
    (1..=2).map(rectangle).collect()
}

fn align_switcher(rectangle_alignment: RectangleAlignment) -> impl Element {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .align(rectangle_alignment.to_align())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_builder(|builder| builder.lazy_entity(lazy_entity.clone()))
        .background_color_signal(
            signal::or!(
                SignalBuilder::from_resource::<Alignment>()
                    .dedupe()
                    .switch(move |In(alignment): In<Alignment>| {
                        match alignment {
                            Alignment::Self_ => SignalBuilder::from_resource::<RectangleSelfAlignment>()
                                .dedupe()
                                .map_in(deref_copied)
                                .left_either(),

                            Alignment::Content => SignalBuilder::from_resource::<RectangleContentAlignment>()
                                .dedupe()
                                .map_in(deref_copied)
                                .right_either(),
                        }
                    })
                    .eq(Some(rectangle_alignment)),
                SignalBuilder::from_lazy_entity(lazy_entity).has_component::<Hovered>(),
            )
            .dedupe()
            .map_bool_in(
                || bevy::color::palettes::css::MIDNIGHT_BLUE,
                || bevy::color::palettes::basic::BLUE,
            )
            .map_in(BackgroundColor::from)
            .map_in(Some),
        )
        .with_node(|mut node| node.padding = UiRect::all(Val::Px(5.)))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(11.67))
                .text(Text(rectangle_alignment.to_string())),
        )
        .on_click(
            move |_: In<_>,
                  alignment: Res<Alignment>,
                  mut self_align: ResMut<RectangleSelfAlignment>,
                  mut content_align: ResMut<RectangleContentAlignment>| {
                match *alignment {
                    Alignment::Self_ => {
                        if **self_align == Some(rectangle_alignment) {
                            **self_align = None;
                        } else {
                            **self_align = Some(rectangle_alignment);
                        }
                    }
                    Alignment::Content => {
                        if **content_align == Some(rectangle_alignment) {
                            **content_align = None;
                        } else {
                            **content_align = Some(rectangle_alignment);
                        }
                    }
                }
            },
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
