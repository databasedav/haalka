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
                    let error_clearer = SignalBuilder::from_resource::<Output>()
                        .map_in(deref_cloned)
                        .dedupe()
                        .map(|_: In<_>, mut commands: Commands| commands.remove_resource::<Error>())
                        .register(world);
                    ui_root()
                        .with_builder(|builder| builder.hold_signals([error_clearer]))
                        .spawn(world);
                },
                camera,
            ),
        )
        .insert_resource(Output(String::new()))
        .insert_resource(Error)
        .run();
}

const BLUE: Color = Color::srgb(91. / 255., 206. / 255., 250. / 255.);
const PINK: Color = Color::srgb(245. / 255., 169. / 255., 184. / 255.);
const FONT_SIZE: f32 = 50.0;
const WIDTH: f32 = 500.;
const BUTTON_SIZE: f32 = WIDTH / 5.;
const GAP: f32 = BUTTON_SIZE / 5.;
const HEIGHT: f32 = BUTTON_SIZE * 5. + GAP * 6.;

#[derive(Resource, Clone)]
struct Error;

#[derive(Resource, Clone, Deref, DerefMut)]
struct Output(String);

fn textable_element(text_signal: impl Signal<Item = impl Into<String> + 'static> + Send + 'static) -> El<Node> {
    El::<Node>::new()
        .with_node(|mut node| node.border = UiRect::all(Val::Px(2.0)))
        .border_color(BorderColor::all(Color::WHITE))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_color(TextColor(Color::WHITE))
                .text_signal(text_signal.map_in(Text::new).map_in(Some)),
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
    let lazy_entity = LazyEntity::new();
    button(symbol)
        .with_builder(|builder| builder.lazy_entity(lazy_entity.clone()))
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .background_color_signal(
            SignalBuilder::from_lazy_entity(lazy_entity)
                .has_component::<Hovered>()
                .map_bool_in(|| BLUE, || PINK)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .on_click(move |In(_), mut output: ResMut<Output>, mut commands: Commands| {
            if symbol == "=" {
                // TryInto::<f64>::try_into(Context::default().evaluate(&output).unwrap()).unwrap();
                if let Ok(result) = Context::<f64>::default().evaluate(&output)
                    && let Some(result) = Decimal::from_f64((result * 100.).round() / 100.)
                {
                    **output = result.normalize().to_string();
                    return;
                }
                commands.insert_resource(Error);
            } else {
                **output += symbol;
            }
        })
}

fn display() -> impl Element {
    textable_element(SignalBuilder::from_resource::<Output>().map_in(deref_cloned))
        .with_builder(|builder| {
            builder.component_signal::<Outline, _>(
                SignalBuilder::from_resource_option::<Error>()
                    .map_in_ref(Option::is_some)
                    .dedupe()
                    .map_true_in(|| Outline::new(Val::Px(4.0), Val::ZERO, bevy::color::palettes::basic::RED.into())),
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
    let output_empty = SignalBuilder::from_resource::<Output>()
        .map_in(deref_cloned)
        .map_in_ref(String::is_empty);
    let lazy_entity = LazyEntity::new();
    button("c")
        .with_builder(|builder| builder.lazy_entity(lazy_entity.clone()))
        .background_color_signal(
            output_empty
                .clone()
                .combine(
                    SignalBuilder::from_lazy_entity(lazy_entity)
                        .has_component::<Hovered>()
                        .dedupe(),
                )
                .map_in(|(output_empty, hovered)| {
                    if output_empty {
                        BLUE
                    } else if hovered {
                        bevy::color::palettes::basic::RED.into()
                    } else {
                        PINK
                    }
                })
                .dedupe()
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .cursor_disableable_signal(CursorIcon::System(SystemCursorIcon::Pointer), output_empty)
        .on_click(|_: In<_>, mut output: ResMut<Output>| output.0.clear())
}

fn ui_root() -> impl Element {
    El::<Node>::new()
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
