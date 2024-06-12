use bevy::prelude::*;
use haalka::*;
use strum::{Display, EnumIter, IntoEnumIterator};

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
        .add_systems(Startup, (ui_root, camera))
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
    fn to_align(&self) -> Align {
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
    El::<NodeBundle>::new()
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
            .map_bool(|| Color::GRAY, || Color::BLACK)
            .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .align_content(Align::center())
        .on_click(move || ALIGNMENT.set(alignment))
        .child(El::<TextBundle>::new().text(text(
            match alignment {
                Alignment::Self_ => "align self",
                Alignment::Content => "align content",
            },
            30.,
        )))
}

fn ui_root(world: &mut World) {
    Column::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_style(|style| style.row_gap = Val::Px(15.))
        .align_content(Align::center())
        .align(Align::center())
        .item(
            Row::<NodeBundle>::new()
                .with_style(|style| style.column_gap = Val::Px(15.))
                .item(container("Column", Column::<NodeBundle>::new().items(rectangles())))
                .item(container("El", El::<NodeBundle>::new().child(rectangle(1))))
                // TODO: is this align content behavior buggy?
                .item(container("Grid", Grid::<NodeBundle>::new().cells(rectangles()))),
        )
        .item(
            Row::<NodeBundle>::new()
                .with_style(|style| style.column_gap = Val::Px(15.))
                .item(
                    Column::<NodeBundle>::new()
                        .with_style(|style| style.row_gap = Val::Px(15.))
                        .item(alignment_button(Alignment::Self_))
                        .item(alignment_button(Alignment::Content)),
                )
                .item(
                    Stack::<NodeBundle>::new()
                        .layers(RectangleAlignment::iter().map(align_switcher))
                        .apply(container_style),
                ),
        )
        .item(
            Row::<NodeBundle>::new()
                .with_style(|style| style.column_gap = Val::Px(15.))
                .item(container("Row", Row::<NodeBundle>::new().items(rectangles())))
                // TODO: is this align content behavior buggy?
                .item(container("Stack", Stack::<NodeBundle>::new().layers(rectangles()))),
        )
        .spawn(world);
}

fn container_style<E: RawElWrapper + Sizeable>(el: E) -> E {
    el.width(Val::Px(278.)).height(Val::Px(200.)).update_raw_el(|raw_el| {
        raw_el
            .insert::<BorderColor>(Color::GRAY.into())
            .with_component::<Style>(|style| {
                style.border = UiRect::all(Val::Px(3.));
            })
    })
}

fn text(text: &str, font_size: f32) -> Text {
    Text::from_section(text, TextStyle { font_size, ..default() })
}

fn container(name: &str, element: impl Element + Sizeable) -> impl Element {
    Column::<NodeBundle>::new()
        .item(
            El::<TextBundle>::new()
                .align(Align::new().center_x())
                .text(text(name, 30.)),
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
                .apply(container_style),
        )
}

fn rectangle(index: i32) -> impl Element {
    let size = 40;
    El::<NodeBundle>::new()
        .width(Val::Px(size as f32))
        .height(Val::Px(size as f32))
        .background_color(BackgroundColor(Color::DARK_GREEN))
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
            El::<TextBundle>::new()
                .align(Align::center())
                .text(text(&index.to_string(), 14.)),
        )
}

fn rectangles() -> Vec<impl Element> {
    (1..=2).map(rectangle).collect()
}

fn align_switcher(rectangle_alignment: RectangleAlignment) -> impl Element {
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    El::<NodeBundle>::new()
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
            .map_bool(|| Color::BLUE.into(), || Color::MIDNIGHT_BLUE.into()),
        )
        .with_style(|style| style.padding = UiRect::all(Val::Px(5.)))
        .child(El::<TextBundle>::new().text(text(&rectangle_alignment.to_string(), 14.)))
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
    commands.spawn(Camera2dBundle::default());
}
