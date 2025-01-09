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

#[derive(Clone, Copy, PartialEq)]
enum Alignment {
    Self_,
    Content,
}

static ALIGNMENT: Lazy<Mutable<Alignment>> = Lazy::new(|| Mutable::new(Alignment::Self_));
static RECTANGLE_SELF_ALIGNMENT: Lazy<Mutable<Option<RectangleAlignment>>> = Lazy::new(default);
static RECTANGLE_CONTENT_ALIGNMENT: Lazy<Mutable<Option<RectangleAlignment>>> = Lazy::new(default);

fn alignment_button(alignment: Alignment) -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .align(Align::center())
        .width(Val::Px(250.))
        .height(Val::Px(80.))
        .background_color_signal(
            signal::or(
                hovered.signal(),
                ALIGNMENT
                    .signal()
                    .map(move |other_alignment| alignment == other_alignment),
            )
            .map_bool(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
            .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .align_content(Align::center())
        .on_click(move || ALIGNMENT.set(alignment))
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
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_node(|mut node| node.row_gap = Val::Px(15.))
        .align_content(Align::center())
        .align(Align::center())
        .item(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.))
                .item(container("Column", Column::<Node>::new().items(rectangles())))
                .item(container("El", El::<Node>::new().child(rectangle(1))))
                // TODO: is this align content behavior buggy?
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
                // TODO: is this align content behavior buggy?
                .item(container("Stack", Stack::<Node>::new().layers(rectangles()))),
        )
}

fn container_node<E: RawElWrapper + Sizeable>(el: E) -> E {
    el.width(Val::Px(278.)).height(Val::Px(200.)).update_raw_el(|raw_el| {
        raw_el
            .insert::<BorderColor>(bevy::color::palettes::basic::GRAY.into())
            .with_component::<Node>(|mut node| {
                node.border = UiRect::all(Val::Px(3.));
            })
    })
}

fn container(name: &str, element: impl Element + Sizeable) -> impl Element {
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
                    ALIGNMENT
                        .signal()
                        .map(|alignment| matches!(alignment, Alignment::Content))
                        .map_true_signal(|| {
                            RECTANGLE_CONTENT_ALIGNMENT
                                .signal_ref(|alignment| alignment.map(|alignment| alignment.to_align()))
                        })
                        .map(Option::flatten),
                )
                .apply(container_node),
        )
}

fn rectangle(index: i32) -> impl Element {
    let size = 40;
    El::<Node>::new()
        .width(Val::Px(size as f32))
        .height(Val::Px(size as f32))
        .background_color(BackgroundColor(bevy::color::palettes::css::DARK_GREEN.into()))
        .align_signal(
            ALIGNMENT
                .signal()
                .map(|alignment| matches!(alignment, Alignment::Self_))
                .map_true_signal(|| {
                    RECTANGLE_SELF_ALIGNMENT.signal_ref(|alignment| alignment.map(|alignment| alignment.to_align()))
                })
                .map(Option::flatten),
        )
        .child(
            El::<Text>::new()
                .align(Align::center())
                .text_font(TextFont::from_font_size(11.67))
                .text(Text::new(index.to_string())),
        )
}

fn rectangles() -> Vec<impl Element> {
    (1..=2).map(rectangle).collect()
}

fn align_switcher(rectangle_alignment: RectangleAlignment) -> impl Element {
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    El::<Node>::new()
        .align(rectangle_alignment.to_align())
        .background_color_signal(
            signal::or(
                ALIGNMENT
                    .signal()
                    .map(|alignment| match alignment {
                        Alignment::Self_ => RECTANGLE_SELF_ALIGNMENT.signal(),
                        Alignment::Content => RECTANGLE_CONTENT_ALIGNMENT.signal(),
                    })
                    .flatten()
                    .map(move |selected_option| selected_option == Some(rectangle_alignment)),
                hovered_signal,
            )
            .map_bool(
                || bevy::color::palettes::basic::BLUE.into(),
                || bevy::color::palettes::css::MIDNIGHT_BLUE.into(),
            ),
        )
        .with_node(|mut node| node.padding = UiRect::all(Val::Px(5.)))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(11.67))
                .text(Text::new(rectangle_alignment.to_string())),
        )
        .hovered_sync(hovered)
        .on_click(move || {
            match ALIGNMENT.get() {
                Alignment::Self_ => &RECTANGLE_SELF_ALIGNMENT,
                Alignment::Content => &RECTANGLE_CONTENT_ALIGNMENT,
            }
            .set(Some(rectangle_alignment));
        })
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}
