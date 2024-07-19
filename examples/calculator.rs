//! Simple calculator. Spurred by https://discord.com/channels/691052431525675048/885021580353237032/1263661461364932639.

use bevy::prelude::*;
use calc::*;
use haalka::prelude::*;
use rust_decimal::prelude::*;

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
        .add_systems(Startup, (camera, ui_root))
        .run();
}

const BLUE: Color = Color::rgb(91. / 255., 206. / 255., 250. / 255.);
const PINK: Color = Color::rgb(245. / 255., 169. / 255., 184. / 255.);
const FONT_SIZE: f32 = 60.0;
const WIDTH: f32 = 500.;
const BUTTON_SIZE: f32 = WIDTH / 5.;
const GAP: f32 = BUTTON_SIZE / 5.;
const HEIGHT: f32 = BUTTON_SIZE * 5. + GAP * 6.;

fn textable_element(text_signal: impl Signal<Item = impl ToString> + Send + 'static) -> El<NodeBundle> {
    El::<NodeBundle>::new()
        .with_style(|mut style| style.border = UiRect::all(Val::Px(2.0)))
        .border_color(BorderColor(Color::WHITE))
        .child(El::<TextBundle>::new().text_signal(text_signal.map(|text| {
            Text::from_section(
                text.to_string(),
                TextStyle {
                    font_size: FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
        })))
}

#[rustfmt::skip]
fn buttons() -> [&'static str; 16] {
    [
        "7", "8", "9", "/",
        "4", "5", "6", "*",
        "1", "2", "3", "-",
        "0", ".", "=", "+",
    ]
}

fn button(symbol: &'static str) -> El<NodeBundle> {
    textable_element(always(symbol))
        .width(Val::Px(BUTTON_SIZE))
        .height(Val::Px(BUTTON_SIZE))
        .align_content(Align::center())
}

fn input_button(symbol: &'static str) -> impl Element {
    let hovered = Mutable::new(false);
    button(symbol)
        .cursor(CursorIcon::Pointer)
        .background_color_signal(hovered.signal().map_bool(|| BLUE, || PINK).map(BackgroundColor))
        .hovered_sync(hovered)
        .on_click(move || {
            let mut output = OUTPUT.lock_mut();
            if symbol == "=" {
                if let Ok(result) = Context::<f64>::default().evaluate(&output) {
                    if let Some(result) = Decimal::from_f64((result * 100.).round() / 100.) {
                        *output = result.normalize().to_string();
                    }
                }
            } else {
                *output += symbol;
            }
        })
}

static OUTPUT: Lazy<Mutable<String>> = Lazy::new(default);

fn display() -> impl Element {
    textable_element(OUTPUT.signal_cloned())
        .with_style(|mut style| {
            style.padding = UiRect::all(Val::Px(GAP));
            style.overflow = Overflow::clip();
        })
        .width(Val::Px(BUTTON_SIZE * 3. + GAP * 2.))
        .height(Val::Px(BUTTON_SIZE))
        .background_color(BackgroundColor(BLUE))
        .align_content(Align::new().right().center_y())
}

fn clear_button() -> impl Element {
    let hovered = Mutable::new(false);
    let output_empty = OUTPUT.signal_ref(String::is_empty).broadcast();
    button("c")
        .background_color_signal(
            map_ref! {
                let output_empty = output_empty.signal(),
                let hovered = hovered.signal() => {
                    if *output_empty {
                        BLUE
                    } else if *hovered {
                        Color::RED
                    } else {
                        PINK
                    }
                }
            }
            .dedupe()
            .map(BackgroundColor),
        )
        .cursor_disableable(CursorIcon::Pointer, output_empty.signal())
        .hovered_sync(hovered)
        .on_click(|| OUTPUT.lock_mut().clear())
}

fn ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .cursor(CursorIcon::Default)
        .align_content(Align::center())
        .child(
            El::<NodeBundle>::new()
                .height(Val::Px(HEIGHT))
                .width(Val::Px(WIDTH))
                .background_color(BackgroundColor(PINK))
                .child(
                    Column::<NodeBundle>::new()
                        .align(Align::center())
                        .with_style(|mut style| {
                            style.row_gap = Val::Px(GAP);
                            style.padding = UiRect::all(Val::Px(GAP));
                        })
                        .item(
                            Row::<NodeBundle>::new()
                                .align(Align::center())
                                .with_style(|mut style| style.column_gap = Val::Px(GAP))
                                .item(clear_button())
                                .item(display()),
                        )
                        .item(
                            Row::<NodeBundle>::new()
                                .multiline()
                                .align_content(Align::center())
                                .with_style(|mut style| {
                                    style.row_gap = Val::Px(GAP);
                                    style.column_gap = Val::Px(GAP);
                                })
                                .width(Val::Percent(100.))
                                .height(Val::Percent(100.))
                                .items(buttons().into_iter().map(input_button)),
                        ),
                ),
        )
        .spawn(world);
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
