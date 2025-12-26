//! Grid of letters that can be scrolled vertically or horizontally.
//!
//! i can't believe it's not scrolling !

mod utils;
use utils::*;

use bevy::{input::mouse::MouseWheel, prelude::*};
use haalka::prelude::*;

fn main() {
    let letters = "abcdefghijklmnopqrstuvwxyz";
    let vertical = (0..5)
        .map(|i| {
            letters
                .chars()
                .cycle()
                .skip(i)
                .take(letters.len())
                .enumerate()
                .map(|(j, letter)| LetterColor {
                    letter: letter.to_string(),
                    color: ROYGBIV[j % 7].into(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let horizontal = (0..5)
        .map(|i| {
            letters
                .chars()
                .cycle()
                .skip(i)
                .take(letters.len())
                .map(|letter| LetterColor {
                    letter: letter.to_string(),
                    color: ROYGBIV[i].into(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    App::new()
        .add_plugins(examples_plugin)
        .insert_resource(Rails { vertical, horizontal })
        .insert_resource(Shifted(false))
        .insert_resource(Cells::new())
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    let cell_data: Vec<_> = {
                        let cells = world.resource::<Cells>();
                        (0..5)
                            .flat_map(|x| (0..5).map(move |y| (x, y, cells.0[x][y].clone())))
                            .collect()
                    };
                    ui_root(cell_data).spawn(world);
                },
                camera,
            ),
        )
        .add_systems(Update, (scroller.run_if(resource_exists::<HoveredCell>), shifter))
        .run();
}

const LETTER_SIZE: f32 = 54.167; // 65 / 1.2
const CELL_SIZE: f32 = 66.;
const NUM_VISIBLE: usize = 5;

#[derive(Clone, Copy)]
enum Scroll {
    Up,
    Down,
}

#[derive(Resource)]
struct HoveredCell(usize, usize);

#[derive(Component, Clone)]
struct CellPosition(usize, usize);

#[derive(Component, Clone, Default)]
struct LetterColorComponent(LetterColor);

#[rustfmt::skip]
fn letter(
    x: usize,
    y: usize,
    initial: LetterColor,
) -> impl Element {
    let lazy_entity = LazyEntity::new();
    let letter_color = SignalBuilder::from_component_lazy::<LetterColorComponent>(lazy_entity.clone())
        .map_in(|LetterColorComponent(lc)| lc);
    let letter = letter_color.clone().map_in(|LetterColor { letter, .. }| letter).dedupe();
    let color = letter_color.map_in(|LetterColor { color, .. }| color).dedupe();
    El::<Node>::new()
    .lazy_entity(lazy_entity.clone())
    .insert(CellPosition(x, y))
    .insert(LetterColorComponent(initial))
    .insert(Pickable::default())
    .with_node(|mut node| {
        node.width = Val::Px(CELL_SIZE);
        node.height = Val::Px(CELL_SIZE);
    })
    .align_content(Align::center())
    .on_hovered_change(move |In((_, data)): In<(Entity, HoverData)>, mut commands: Commands| {
        if data.hovered {
            commands.insert_resource(HoveredCell(x, y));
        }
    })
    .child(
        El::<Text>::new()
        .text_font(TextFont::from_font_size(LETTER_SIZE))
        .text_color_signal(color.map_in(TextColor).map_in(Some))
        .text_signal(letter.map_in(Text).map_in(Some))
    )
}

#[derive(Clone, Default, PartialEq)]
struct LetterColor {
    letter: String,
    color: Color,
}

#[derive(Resource)]
struct Rails {
    vertical: Vec<Vec<LetterColor>>,
    horizontal: Vec<Vec<LetterColor>>,
}

const ROYGBIV: &[Srgba] = &[
    bevy::color::palettes::css::RED,
    bevy::color::palettes::css::ORANGE,
    bevy::color::palettes::css::YELLOW,
    bevy::color::palettes::css::GREEN,
    bevy::color::palettes::css::BLUE,
    bevy::color::palettes::css::INDIGO,
    bevy::color::palettes::css::VIOLET,
];

#[derive(Resource)]
struct Cells([[LetterColor; 5]; 5]);

impl Cells {
    fn new() -> Self {
        let letters = "abcdefghijklmnopqrstuvwxyz";
        let mut cells = [[(); 5]; 5].map(|row| row.map(|_| LetterColor::default()));
        for i in 0..5 {
            for (j, letter) in letters.chars().skip(i).take(5).enumerate() {
                cells[i][j] = LetterColor {
                    letter: letter.to_string(),
                    color: ROYGBIV[i].into(),
                };
            }
        }
        Self(cells)
    }
}

fn ui_root(cell_data: Vec<(usize, usize, LetterColor)>) -> impl Element {
    let shifted = SignalBuilder::from_resource::<Shifted>()
        .map_in(|Shifted(s)| s)
        .dedupe();
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Grid::<Node>::new()
                .insert(Pickable::default())
                .on_hovered_change(|In((_, data)): In<(Entity, HoverData)>, mut commands: Commands| {
                    if !data.hovered {
                        commands.remove_resource::<HoveredCell>();
                    }
                })
                .cursor_signal(
                    shifted
                        .map_bool_in(
                            || SystemCursorIcon::EwResize,
                            || SystemCursorIcon::NsResize,
                        )
                        .map_in(CursorIcon::System)
                        .dedupe(),
                )
                .row_wrap_cell_width(CELL_SIZE)
                .with_node(|mut node| {
                    node.width = Val::Px(CELL_SIZE * NUM_VISIBLE as f32);
                    node.height = Val::Px(CELL_SIZE * NUM_VISIBLE as f32);
                })
                .align(Align::center())
                .cells(cell_data.into_iter().map(|(x, y, lc)| letter(x, y, lc))),
        )
}

fn scroller(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    hovered_cell: Res<HoveredCell>,
    mut rails: ResMut<Rails>,
    shifted: Res<Shifted>,
    mut cells: ResMut<Cells>,
    mut letter_colors: Query<(&CellPosition, &mut LetterColorComponent)>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        let scroll = if mouse_wheel_event.y.is_sign_negative() {
            Scroll::Up
        } else {
            Scroll::Down
        };
        let HoveredCell(x, y) = *hovered_cell;
        let Rails { vertical, horizontal } = &mut *rails;
        match scroll {
            Scroll::Up => {
                if shifted.0 {
                    horizontal[x].rotate_left(1);
                    for (v, h) in vertical.iter_mut().zip(horizontal[x].iter()) {
                        v[x] = h.clone();
                    }
                    for (j, v) in horizontal[x].iter().take(5).enumerate() {
                        cells.0[x][j] = v.clone();
                    }
                } else {
                    vertical[y].rotate_left(1);
                    for (h, v) in horizontal.iter_mut().zip(vertical[y].iter()) {
                        h[y] = v.clone();
                    }
                    for (i, v) in vertical[y].iter().take(5).enumerate() {
                        cells.0[i][y] = v.clone();
                    }
                }
            }
            Scroll::Down => {
                if shifted.0 {
                    horizontal[x].rotate_right(1);
                    for (v, h) in vertical.iter_mut().zip(horizontal[x].iter()) {
                        v[x] = h.clone();
                    }
                    for (j, v) in horizontal[x].iter().take(5).enumerate() {
                        cells.0[x][j] = v.clone();
                    }
                } else {
                    vertical[y].rotate_right(1);
                    for (h, v) in horizontal.iter_mut().zip(vertical[y].iter()) {
                        h[y] = v.clone();
                    }
                    for (i, v) in vertical[y].iter().take(5).enumerate() {
                        cells.0[i][y] = v.clone();
                    }
                }
            }
        }
        // Update the LetterColorComponent on the ECS entities
        for (pos, mut lc) in letter_colors.iter_mut() {
            let CellPosition(px, py) = *pos;
            lc.0 = cells.0[px][py].clone();
        }
    }
}

#[derive(Resource, Clone, Copy)]
struct Shifted(bool);

fn shifter(keys: Res<ButtonInput<KeyCode>>, mut shifted: ResMut<Shifted>) {
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) {
        shifted.0 = true;
    } else if keys.just_released(KeyCode::ShiftLeft) || keys.just_released(KeyCode::ShiftRight) {
        shifted.0 = false;
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
