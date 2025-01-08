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

const LETTER_SIZE: f32 = 65.;

static SHIFTED: Lazy<Mutable<bool>> = Lazy::new(default);

fn letter(letter: &str, color: Color) -> impl Element {
    El::<Text>::new().text(Text::from_section(
        letter,
        TextStyle {
            font_size: LETTER_SIZE,
            color,
            ..default()
        },
    ))
}

fn letter_column(rotate: usize, color: Color) -> impl Element {
    let hovered = Mutable::new(false);
    Column::<Node>::new()
        .height(Val::Px(5. * LETTER_SIZE))
        .mutable_viewport(Overflow::clip_y(), LimitToBody::Vertical)
        .on_scroll_with_system_disableable_signal(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(LETTER_SIZE)
                .into_system(),
            signal::or(signal::not(hovered.signal()), SHIFTED.signal()),
        )
        .with_style(move |mut style| style.top = Val::Px(-LETTER_SIZE * rotate as f32))
        .hovered_sync(hovered)
        .items(
            "abcdefghijklmnopqrstuvwxyz"
                .chars()
                .map(move |c| letter(&c.to_string(), color)),
        )
}

fn ui_root() -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_style(|mut style: Mut<'_, Style>| {
                    style.column_gap = Val::Px(30.);
                    style.padding = UiRect::horizontal(Val::Px(7.5));
                })
                .width(Val::Px(300.))
                .mutable_viewport(Overflow::clip_x(), LimitToBody::Horizontal)
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
    commands.spawn(Camera2dBundle::default());
}
