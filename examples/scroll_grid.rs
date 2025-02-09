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
                .take(26)
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
                .take(26)
                .map(|letter| LetterColor {
                    letter: letter.to_string(),
                    color: ROYGBIV[i].into(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
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
        .add_systems(Update, (scroller.run_if(resource_exists::<HoveredCell>), shifter))
        .insert_resource(Rails { vertical, horizontal })
        .insert_resource(Shifted(false))
        .run();
}

const LETTER_SIZE: f32 = 54.167; // 65 / 1.2

#[derive(Clone, Copy)]
enum Scroll {
    Up,
    Down,
}

#[derive(Resource)]
struct HoveredCell(usize, usize);

#[rustfmt::skip]
fn letter(
    x: usize,
    y: usize,
    letter_color: impl Signal<Item = LetterColor> + Send + Sync + 'static,
) -> impl Element {
    let letter_color = letter_color.broadcast();
    let letter = letter_color.signal_ref(|LetterColor { letter, .. }| letter.clone());
    let color = letter_color.signal_ref(|LetterColor { color, .. }| *color);
    El::<Text>::new()
    .on_hovered_change(move |is_hovered| {
        if is_hovered {
            async_world().insert_resource(HoveredCell(x, y)).apply(spawn).detach()
        }
    })
    .text_font(TextFont::from_font_size(LETTER_SIZE))
    .text_color_signal(color.map(Into::into))
    .text_signal(letter.map(Text))
}

#[derive(Clone, Default)]
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

static CELLS: Lazy<Vec<Vec<Mutable<LetterColor>>>> = Lazy::new(|| {
    let cells = (0..5)
        .map(|_| (0..5).map(|_| Mutable::new(default())).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let letters = "abcdefghijklmnopqrstuvwxyz";
    for i in 0..5 {
        for (j, letter) in letters.chars().skip(i).take(5).enumerate() {
            cells[i][j].set(LetterColor {
                letter: letter.to_string(),
                color: ROYGBIV[i].into(),
            });
        }
    }
    cells
});

fn ui_root() -> impl Element {
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Grid::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.))
                .on_hovered_change(move |is_hovered| {
                    if !is_hovered {
                        async_world().remove_resource::<HoveredCell>().apply(spawn).detach()
                    }
                })
                .row_wrap_cell_width(48.)
                .width(Val::Px(300.))
                .height(Val::Px(5. * LETTER_SIZE))
                .align(Align::center())
                .cells(CELLS.iter().enumerate().flat_map(|(x, cells)| {
                    cells
                        .iter()
                        .enumerate()
                        .map(move |(y, cell)| letter(x, y, cell.signal_cloned()))
                })),
        )
}

fn scroller(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hovered_cell: Res<HoveredCell>,
    mut rails: ResMut<Rails>,
    shifted: Res<Shifted>,
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
                    for (cell, v) in CELLS[x].iter().zip(horizontal[x].iter()) {
                        cell.set(v.clone());
                    }
                } else {
                    vertical[y].rotate_left(1);
                    for (h, v) in horizontal.iter_mut().zip(vertical[y].iter()) {
                        h[y] = v.clone();
                    }
                    for (cell, v) in CELLS.iter().zip(vertical[y].iter()) {
                        cell[y].set(v.clone());
                    }
                }
            }
            Scroll::Down => {
                if shifted.0 {
                    horizontal[x].rotate_right(1);
                    for (v, h) in vertical.iter_mut().zip(horizontal[x].iter()) {
                        v[x] = h.clone();
                    }
                    for (cell, v) in CELLS[x].iter().zip(horizontal[x].iter()) {
                        cell.set(v.clone());
                    }
                } else {
                    vertical[y].rotate_right(1);
                    for (h, v) in horizontal.iter_mut().zip(vertical[y].iter()) {
                        h[y] = v.clone();
                    }
                    for (cell, v) in CELLS.iter().zip(vertical[y].iter()) {
                        cell[y].set(v.clone());
                    }
                }
            }
        }
    }
}

#[derive(Resource)]
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
