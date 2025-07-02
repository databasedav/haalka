//! Scrollable row of scrollable letter columns. Inspired by <https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs>.

mod utils;
use utils::*;

use bevy::prelude::*;
use haalka::prelude::*;

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
        .add_systems(Update, shifter)
        .run();
}

const LETTER_SIZE: f32 = 54.167; // 65 / 1.2
const COMPUTED_SIZE: f32 = 66.; // TODO: how/y tho ?

static SHIFTED: LazyLock<Mutable<bool>> = LazyLock::new(default);

fn letter(letter: String, color: Color) -> impl Element {
    El::<Text>::new()
        .text_font(TextFont::from_font_size(LETTER_SIZE))
        .text_color(TextColor(color))
        .text(Text::new(letter))
}

fn letter_column(rotate: usize, color: Color) -> impl Element {
    let hovered = Mutable::new(false);
    Column::<Node>::new()
        .with_node(|mut node| node.height = Val::Px(5. * COMPUTED_SIZE))
        .mutable_viewport(haalka::prelude::Axis::Vertical)
        .on_scroll_with_system_disableable_signal(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(COMPUTED_SIZE)
                .into_system(),
            signal::or(signal::not(hovered.signal()), SHIFTED.signal()),
        )
        .with_scroll_position(move |mut scroll_position| scroll_position.offset_y = COMPUTED_SIZE * rotate as f32)
        .hovered_sync(hovered)
        .items(
            "abcdefghijklmnopqrstuvwxyz"
                .chars()
                .map(move |c| letter(c.to_string(), color)),
        )
}

fn ui_root() -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| {
                    node.width = Val::Px(300.);
                    node.column_gap = Val::Px(30.);
                    node.padding = UiRect::horizontal(Val::Px(7.5));
                })
                .mutable_viewport(haalka::prelude::Axis::Horizontal)
                .on_scroll_with_system_disableable_signal(
                    BasicScrollHandler::new()
                        .direction(ScrollDirection::Horizontal)
                        // TODO: special handler for auto discrete like rectray https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs
                        .pixels(63.)
                        .into_system(),
                    signal::not(signal::and(hovered.signal(), SHIFTED.signal())),
                )
                .hovered_sync(hovered)
                .items(
                    [
                        bevy::color::palettes::css::RED,
                        bevy::color::palettes::css::ORANGE,
                        bevy::color::palettes::css::YELLOW,
                        bevy::color::palettes::css::GREEN,
                        bevy::color::palettes::css::BLUE,
                        bevy::color::palettes::css::INDIGO,
                        bevy::color::palettes::css::VIOLET,
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(i, color)| letter_column(i, color.into())),
                ),
        )
}

fn shifter(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) {
        SHIFTED.set_neq(true);
    } else if keys.just_released(KeyCode::ShiftLeft) || keys.just_released(KeyCode::ShiftRight) {
        SHIFTED.set_neq(false);
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
