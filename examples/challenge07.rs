//! - A dropdown for the type of bug (UI/cosmetics/gameplay).
//! - A one-line text input for the bug title.
//! - A multi-line text input for the bug description.
//! - The text editing should have the following features:
//!   - Cursor, which can be moved with arrow keys and mouse click.
//!   - Text selection.
//!   - Copy/paste/cut with the usual shortcuts.

mod utils;
use utils::*;

use std::convert::identity;

use bevy::prelude::*;
use bevy_cosmic_edit::{
    cosmic_text::{Family, FamilyOwned},
    CosmicBackgroundColor, CosmicWrap, CursorColor, FontWeight, MaxLines,
};
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
        .run();
}

fn ui_root() -> impl Element {
    El::<NodeBundle>::new()
        .ui_root()
        .cursor(CursorIcon::Default)
        .height(Val::Percent(100.))
        .width(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Column::<NodeBundle>::new()
                .height(Val::Percent(80.))
                .width(Val::Percent(60.))
                .item({
                    let focus = Mutable::new(false);
                    TextInput::new()
                        .width(Val::Px(100.))
                        .height(Val::Px(30.))
                        .mode(CosmicWrap::InfiniteLine)
                        .font_size(16.)
                        .max_lines(MaxLines(1))
                        .attrs(
                            TextAttrs::new()
                                .family(FamilyOwned::new(Family::Name("Fira Mono")))
                                .weight(FontWeight::MEDIUM),
                        )
                        .scroll_disabled()
                        .cursor_color_signal(
                            focus
                                .signal()
                                .map_bool(|| Color::WHITE, || Color::BLACK)
                                .map(CursorColor),
                        )
                        // TODO: flip colors once https://github.com/Dimchikkk/bevy_cosmic_edit/issues/144
                        .fill_color_signal(
                            focus
                                .signal()
                                .map_bool(|| Color::BLACK, || Color::WHITE)
                                .map(CosmicBackgroundColor),
                        )
                        .attrs(
                            TextAttrs::new()
                                .color_signal(focus.signal().map_bool(|| Color::WHITE, || Color::BLACK).map(Some)),
                        )
                        .focus_signal(focus.signal())
                        .on_focused_change(clone!((focus) move |is_focused| {
                            focus.set_neq(is_focused);
                        }))
                    // .text_signal(string.signal_cloned())
                    // .on_change_sync(string)
                })
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|mut style| style.column_gap = Val::Px(15.))
                        .item(El::<TextBundle>::new().text(text_with_size("bug report", 50.)))
                        .item(dropdown(["UI", "cosmetics", "gameplay"], Some("type"))),
                ),
        )
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

const BASE_PADDING: f32 = 5.;

fn button() -> El<NodeBundle> {
    let (pressed, pressed_signal) = Mutable::new_and_signal(false);
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    let pressed_hovered_broadcaster =
        map_ref!(pressed_signal, hovered_signal => (*pressed_signal, *hovered_signal)).broadcast();
    let border_color_signal = {
        pressed_hovered_broadcaster
            .signal()
            .map(|(pressed, hovered)| {
                if pressed {
                    bevy::color::palettes::basic::RED.into()
                } else if hovered {
                    Color::WHITE
                } else {
                    Color::BLACK
                }
            })
            .map(BorderColor)
    };
    let background_color_signal = {
        pressed_hovered_broadcaster
            .signal()
            .map(|(pressed, hovered)| {
                if pressed {
                    PRESSED_BUTTON
                } else if hovered {
                    HOVERED_BUTTON
                } else {
                    NORMAL_BUTTON
                }
            })
            .map(BackgroundColor)
    };
    El::<NodeBundle>::new()
        .width(Val::Px(150.0))
        .height(Val::Px(65.))
        .with_style(|mut style| style.border = UiRect::all(Val::Px(5.0)))
        .align_content(Align::center())
        .border_color_signal(border_color_signal)
        .background_color_signal(background_color_signal)
        .hovered_sync(hovered)
        .cursor_disableable_signal(CursorIcon::Grabbing, pressed.signal().dedupe())
        .pressed_sync(pressed)
}

fn x_button(on_click: impl FnMut() + Send + Sync + 'static) -> impl Element {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
        .background_color(BackgroundColor(Color::NONE))
        // stop propagation because otherwise clearing the dropdown will drop down the
        // options too; the x should eat the click
        .on_click_stop_propagation(on_click)
        .child(
            El::<TextBundle>::new().text(text("x")).on_signal_with_text(
                hovered
                    .signal()
                    .map_bool(|| bevy::color::palettes::basic::RED.into(), || Color::WHITE),
                |mut text, color| {
                    if let Some(section) = text.sections.first_mut() {
                        section.style.color = color;
                    }
                },
            ),
        )
        .hovered_sync(hovered)
}

fn dropdown(options: impl IntoIterator<Item = &'static str>, placeholder: Option<&'static str>) -> impl Element {
    let selected: Mutable<Option<String>> = Mutable::new(None);
    let show_dropdown = Mutable::new(false);
    let options = MutableVec::from(options.into_iter().map(ToString::to_string).collect::<Vec<_>>());
    button()
        .child(
            Stack::<NodeBundle>::new()
                .width(Val::Percent(100.))
                .with_style(|mut style| style.padding = UiRect::horizontal(Val::Px(BASE_PADDING)))
                .layer(
                    El::<TextBundle>::new().align(Align::new().left()).text_signal(
                        selected
                            .signal_cloned()
                            .map_option(identity, move || placeholder.unwrap_or_default().to_string())
                            .map(text),
                    ),
                )
                .layer(
                    Row::<NodeBundle>::new()
                        .with_style(|mut style| style.column_gap = Val::Px(BASE_PADDING))
                        .align(Align::new().right())
                        .item_signal({
                            selected.signal_ref(Option::is_some).dedupe().map_true(
                                clone!((selected) move || x_button(clone!((selected) move || { selected.take(); }))),
                            )
                        })
                        .item(
                            El::<TextBundle>::new()
                                // TODO: need to figure out to rotate in place (around center)
                                // .on_signal_with_transform(show_dropdown.signal(), |transform, showing| {
                                //     transform.rotate_around(Vec3::X, Quat::from_rotation_z((if showing { 180.0f32 }
                                // else { 0. }).to_radians())); })
                                .text(text("v")),
                        ),
                ),
        )
        // TODO: this should be element below signal
        .child_signal(
            show_dropdown
                .signal()
                .map_true(clone!((options, show_dropdown, selected) move || {
                    Column::<NodeBundle>::new()
                    .width(Val::Percent(100.))
                    .with_style(|mut style| {
                        style.position_type = PositionType::Absolute;
                        style.top = Val::Percent(100.);
                    })
                    .items_signal_vec(
                        options.signal_vec_cloned()
                        .filter_signal_cloned(clone!((selected) move |option| {
                            selected.signal_ref(clone!((option) move |selected_option| {
                                selected_option.as_ref() != Some(&option)
                            }))
                            .dedupe()
                        }))
                        .map(clone!((selected, show_dropdown) move |option| {
                            button()
                            .child(El::<TextBundle>::new().text(text(&option)))
                            .on_click(
                                clone!((selected, show_dropdown, option) move || {
                                    selected.set_neq(Some(option.clone()));
                                    flip(&show_dropdown);
                                })
                            )
                        }))
                    )
                })),
        )
}

fn text_with_size(text: impl ToString, size: f32) -> Text {
    Text::from_section(
        text.to_string(),
        TextStyle {
            font_size: size,
            ..default()
        },
    )
}

const DEFAULT_FONT_SIZE: f32 = 20.;

fn text(text: impl ToString) -> Text {
    text_with_size(text, DEFAULT_FONT_SIZE)
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
