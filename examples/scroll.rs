use bevy::prelude::*;
use haalka::*;

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
        .add_systems(Update, shifter)
        .run();
}

const LETTER_SIZE: f32 = 65.;

static SHIFTED: Lazy<Mutable<bool>> = Lazy::new(default);

fn letter(letter: &str, color: Color) -> impl Element {
    El::<TextBundle>::new().text(Text::from_section(
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
    Column::<NodeBundle>::new()
        .height(Val::Px(5. * LETTER_SIZE))
        .scrollable(
            ScrollabilitySettings {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
                scroll_handler: BasicScrollHandler::new()
                    .direction(ScrollDirection::Vertical)
                    .pixels(LETTER_SIZE)
                    .into(),
            },
            signal::and(signal::not(SHIFTED.signal()), hovered.signal()),
        )
        .with_style(move |style| style.top = Val::Px(-LETTER_SIZE * rotate as f32))
        .hovered_sync(hovered)
        .items(
            "abcdefghijklmnopqrstuvwxyz"
                .chars()
                .map(move |c| letter(&c.to_string(), color)),
        )
}

fn ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<NodeBundle>::new()
                .with_style(|style| style.column_gap = Val::Px(30.))
                .width(Val::Px(300.))
                .scrollable(
                    ScrollabilitySettings {
                        flex_direction: FlexDirection::Row,
                        overflow: Overflow::clip_x(),
                        scroll_handler: BasicScrollHandler::new()
                            .direction(ScrollDirection::Horizontal)
                            // TODO: special handler for auto discrete like rectray https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs
                            .pixels(63.)
                            .into(),
                    },
                    SHIFTED.signal(),
                )
                .items(
                    [
                        Color::RED,
                        Color::ORANGE,
                        Color::YELLOW,
                        Color::GREEN,
                        Color::BLUE,
                        Color::INDIGO,
                        Color::VIOLET,
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(i, color)| letter_column(i, color)),
                ),
        )
        .spawn(world);
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
