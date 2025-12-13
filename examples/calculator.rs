//! Simple calculator. Spurred by <https://discord.com/channels/691052431525675048/885021580353237032/1263661461364932639>.

mod utils;
use utils::*;

use bevy::prelude::*;
use calc::*;
use haalka::prelude::*;
use rust_decimal::prelude::*;

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

const BLUE: Color = Color::srgb(91. / 255., 206. / 255., 250. / 255.);
const PINK: Color = Color::srgb(245. / 255., 169. / 255., 184. / 255.);
const FONT_SIZE: f32 = 50.0;
const WIDTH: f32 = 500.;
const BUTTON_SIZE: f32 = WIDTH / 5.;
const GAP: f32 = BUTTON_SIZE / 5.;
const HEIGHT: f32 = BUTTON_SIZE * 5. + GAP * 6.;

fn textable_element(text_signal: impl Signal<Item = impl Into<String> + 'static> + Send + 'static) -> El<Node> {
    El::<Node>::new()
        .with_node(|mut node| node.border = UiRect::all(Val::Px(2.0)))
        .border_color(BorderColor::all(Color::WHITE))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_color(TextColor(Color::WHITE))
                .text_signal(text_signal.map(Text::new)),
        )
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

fn button(symbol: &'static str) -> El<Node> {
    textable_element(SignalBuilder::always(symbol))
        .with_node(|mut node| {
            node.width = Val::Px(BUTTON_SIZE);
            node.height = Val::Px(BUTTON_SIZE);
        })
        .align_content(Align::center())
}

fn input_button(symbol: &'static str) -> impl Element {
    let hovered = Mutable::new(false);
    let f = move || {
        if symbol == "=" {
            let mut output = OUTPUT.lock_mut();
            if let Some(result) = Context::default()
                .evaluate(&output)
                .ok()
                .and_then(|value| TryInto::<f64>::try_into(value).ok())
                && let Some(result) = Decimal::from_f64((result * 100.).round() / 100.)
            {
                *output = result.normalize().to_string();
                return;
            }
            ERROR.set_neq(true);
        } else {
            *OUTPUT.lock_mut() += symbol;
        }
    };
    let lazy_entity = LazyEntity::new();
    button(symbol)
        .hoverable()
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .background_color_signal(SignalBuilder::from_component_lazy::<Hovered>(lazy_entity).map_in(deref_copied).map_bool_in(|| BLUE, || PINK).map_in(BackgroundColor).map_in(Some))
        .on_click(f)
}

static OUTPUT: LazyLock<Mutable<String>> = LazyLock::new(default);
static ERROR: LazyLock<Mutable<bool>> = LazyLock::new(default);

fn display() -> impl Element {
    textable_element(OUTPUT.signal_cloned())
        .update_raw_el(|raw_el| {
            raw_el.component_signal::<Outline, _>(
                ERROR
                    .signal()
                    .map_true(|| Outline::new(Val::Px(4.0), Val::ZERO, bevy::color::palettes::basic::RED.into())),
            )
        })
        .with_node(|mut node| {
            node.width = Val::Px(BUTTON_SIZE * 3. + GAP * 2.);
            node.height = Val::Px(BUTTON_SIZE);
            node.padding = UiRect::all(Val::Px(GAP));
            node.overflow = Overflow::clip();
        })
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
                        bevy::color::palettes::basic::RED.into()
                    } else {
                        PINK
                    }
                }
            }
            .dedupe()
            .map(BackgroundColor),
        )
        .cursor_disableable_signal(CursorIcon::System(SystemCursorIcon::Pointer), output_empty.signal())
        .hovered_sync(hovered)
        .on_click(|| OUTPUT.lock_mut().clear())
}

fn ui_root() -> impl Element {
    let error_clearer = OUTPUT.signal_ref(|_| ERROR.set_neq(false)).to_future().apply(spawn);
    El::<Node>::new()
        .update_raw_el(|raw_el| raw_el.hold_tasks([error_clearer]))
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Column::<Node>::new()
                .background_color(BackgroundColor(PINK))
                .align(Align::center())
                .with_node(|mut node| {
                    node.height = Val::Px(HEIGHT);
                    node.width = Val::Px(WIDTH);
                    node.row_gap = Val::Px(GAP);
                    node.padding = UiRect::all(Val::Px(GAP));
                })
                .item(
                    Row::<Node>::new()
                        .align(Align::center())
                        .with_node(|mut node| node.column_gap = Val::Px(GAP))
                        .item(clear_button())
                        .item(display()),
                )
                .item(
                    Row::<Node>::new()
                        .multiline()
                        .align_content(Align::center())
                        .with_node(|mut node| {
                            node.row_gap = Val::Px(GAP);
                            node.column_gap = Val::Px(GAP);
                        })
                        .items(buttons().into_iter().map(input_button)),
                ),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
