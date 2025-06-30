//! Text inputs, scrolling/viewport control, and reactive lists.
//!
//! promises made promises kept ! <https://discord.com/channels/691052431525675048/1192585689460658348/1193431789465776198>
//! (yes i take requests)

mod utils;
use bevy_input_focus::InputFocus;
use bevy_ui_text_input::TextInputMode;
use utils::*;

use std::{cmp::Ordering, ops::Not, time::Duration};

use bevy::prelude::*;
use haalka::{
    prelude::*,
    viewport_mutable::{LogicalRect, MutableViewport},
};

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
        .add_systems(
            Update,
            (
                tabber,
                escaper,
                focus_scroller.run_if(resource_changed::<InputFocus>),
            ),
        )
        .add_observer(sort_one)
        .run();
}

const INPUT_HEIGHT: f32 = 40.;
const INPUT_WIDTH: f32 = 200.;
const STARTING_SORTED_BY: KeyValue = KeyValue::Key;
const PADDING: f32 = 10.;
static DARK_GRAY: LazyLock<Color> = LazyLock::new(|| Srgba::gray(0.25).into());

static PAIRS: LazyLock<MutableVec<RowData>> = LazyLock::new(|| {
    let mut pairs = [
        ("lorem", "ipsum"),
        ("dolor", "sit"),
        ("amet", "consectetur"),
        ("adipiscing", "elit"),
        ("sed", "do"),
        ("eiusmod", "tempor"),
        ("incididunt", "ut"),
        ("labore", "et"),
        ("dolore", "magna"),
        ("aliqua", "ut"),
        ("enim", "ad"),
        ("minim", "veniam"),
        ("quis", "nostrud"),
        ("exercitation", "ullamco"),
        ("laboris", "nisi"),
        ("ut", "aliquip"),
        ("ex", "ea"),
        ("commodo", "consequat"),
        ("duis", "aute"),
        ("irure", "dolor"),
        ("in", "reprehenderit"),
        ("in", "voluptate"),
        ("velit", "esse"),
        ("cillum", "dolore"),
        ("eu", "fugiat"),
        ("nulla", "pariatur"),
        ("excepteur", "sint"),
        ("occaecat", "cupidatat"),
        ("non", "proident"),
        ("sunt", "in"),
    ]
    .into_iter()
    .collect::<Vec<_>>();
    match STARTING_SORTED_BY {
        KeyValue::Key => {
            pairs.sort_by_key(|&(key, _)| key);
        }
        KeyValue::Value => {
            pairs.sort_by_key(|&(_, value)| value);
        }
    }
    pairs
        .into_iter()
        .map(|(key, value)| RowData {
            key: TextInputData::new(key),
            value: TextInputData::new(value),
        })
        .collect::<Vec<_>>()
        .into()
});

#[derive(Clone, Copy, PartialEq)]
enum KeyValue {
    Key,
    Value,
}

static SORT_BY: LazyLock<Mutable<KeyValue>> = LazyLock::new(|| Mutable::new(STARTING_SORTED_BY));

#[derive(Clone)]
struct TextInputData {
    string: Mutable<String>,
    focus: Mutable<bool>,
}

#[derive(Clone)]
struct RowData {
    key: TextInputData,
    value: TextInputData,
}

impl TextInputData {
    fn new(string: &str) -> Self {
        Self {
            string: Mutable::new(string.to_string()),
            focus: Mutable::new(false),
        }
    }
}

fn text_input(
    index_option: ReadOnlyMutable<Option<usize>>,
    string: Mutable<String>,
    focus: Mutable<bool>,
) -> impl Element {
    El::<Node>::new()
        .apply(border_radius_style(10.))
        .height(Val::Px(INPUT_HEIGHT))
        .width(Val::Px(INPUT_WIDTH))
        .background_color_signal(
            focus
                .signal()
                .map_bool(|| Color::WHITE, || *DARK_GRAY)
                .map(BackgroundColor),
        )
        .with_node(|mut node| node.overflow = Overflow::clip())
        .cursor(CursorIcon::System(SystemCursorIcon::Text))
        .on_click(clone!((focus) move || focus.set_neq(true)))
        .on_click_outside_with_system(|In((entity, _)), mut input_focus: ResMut<InputFocus>, children: Query<&Children>| {
            if input_focus.0 == Some(children.get(entity).unwrap().iter().next().unwrap()) {
                input_focus.0 = None;
            }
        })
        .child(
            TextInput::new()
                .align(Align::new().center_y())
                .with_node(|mut node| node.left = Val::Px(PADDING))
                .width(Val::Px(INPUT_WIDTH - PADDING * 2.))
                .height(Val::Px(INPUT_HEIGHT - PADDING * 2. + 5.))
                .with_text_input_node(|mut node| {
                    node.mode = TextInputMode::SingleLine;
                    // TODO: https://github.com/ickshonpe/bevy_ui_text_input/issues/10
                    // node.justification = JustifyText::Center;
                })
                .text_color_signal(focus.signal().map_bool(|| Color::BLACK, || Color::WHITE).map(TextColor))
                .focus_signal(focus.signal())
                .on_focused_change_with_system(
                    clone!((focus) move |In((_, is_focused)): In<(Entity, bool)>, mut commands: Commands| {
                        if !is_focused && let Some(index) = index_option.get() {
                            commands.trigger(MaybeChanged(index));
                        }
                        focus.set_neq(is_focused);
                    }),
                )
                .text_signal(string.signal_cloned())
                .on_change_sync(string)
        )
}

fn clear_focus() {
    for RowData { key, value } in PAIRS.lock_ref().iter() {
        key.focus.set_neq(false);
        value.focus.set_neq(false);
    }
}

fn sort_by_text_element() -> impl Element {
    El::<Text>::new()
        .text_font(TextFont::from_font_size(60.))
        .text_color(TextColor(Color::WHITE))
        .text(Text::new("sort by"))
}

fn border_radius_style<E: Element>(border_radius: f32) -> impl FnOnce(E) -> E {
    move |el| el.update_raw_el(|raw_el| raw_el.insert(BorderRadius::all(Val::Px(border_radius))))
}

fn sort_button(sort_by: KeyValue) -> impl Element {
    let hovered = Mutable::new(false);
    let selected = SORT_BY.signal().map(move |cur| cur == sort_by).broadcast();
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(35.))
        .align(Align::new().right())
        .item_signal(selected.signal().map_true(sort_by_text_element))
        .item(
            El::<Node>::new()
                .apply(border_radius_style(20.))
                .width(Val::Px(200.))
                .height(Val::Px(80.))
                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                .background_color_signal(
                    signal::or(hovered.signal(), selected.signal())
                        .map_bool(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
                        .map(BackgroundColor),
                )
                .hovered_sync(hovered)
                .align_content(Align::center())
                .on_click(move || {
                    let mut lock = SORT_BY.lock_mut();
                    if *lock != sort_by {
                        *lock = sort_by;
                        match sort_by {
                            KeyValue::Key => {
                                let mut lock = PAIRS.lock_mut();
                                let mut values = lock.to_vec();
                                // TODO: avoid cloning
                                values.sort_by_key(|RowData { key, .. }| key.string.get_cloned());
                                // put empty strings at the end
                                values.sort_by_key(|RowData { key, .. }| key.string.lock_ref().is_empty());
                                lock.replace_cloned(values);
                            }
                            KeyValue::Value => {
                                let mut lock = PAIRS.lock_mut();
                                let mut values = lock.to_vec();
                                // TODO: avoid cloning
                                values.sort_by_key(|RowData { value, .. }| value.string.get_cloned());
                                // put empty strings at the end
                                values.sort_by_key(|RowData { value, .. }| value.string.lock_ref().is_empty());
                                lock.replace_cloned(values);
                            }
                        }
                    }
                })
                .child(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(60.))
                        .text_color(TextColor(Color::WHITE))
                        .text(Text::new(match sort_by {
                            KeyValue::Key => "key",
                            KeyValue::Value => "value",
                        })),
                ),
        )
}

#[derive(Clone, Copy, Event)]
struct MaybeChanged(usize);

/// Checks if an item at a given index is correctly sorted relative to its direct neighbors.
/// This is a fast-path check to prevent unnecessary moves and stop stability loops.
fn is_sorted_at(pairs: &MutableVecLockMut<RowData>, index: usize, get_string: &impl Fn(&RowData) -> String) -> bool {
    let item_string = get_string(&pairs[index]);

    // Check against the previous item
    if index > 0 {
        let prev_string = get_string(&pairs[index - 1]);
        // The comparison logic: `item` should be >= `prev`.
        let ordering_correct = match (item_string.is_empty(), prev_string.is_empty()) {
            (true, false) => true,           // Empty after non-empty is correct.
            (false, true) => false,          // Non-empty after empty is INCORRECT.
            _ => item_string >= prev_string, // Otherwise, check lexicographically.
        };
        if !ordering_correct {
            return false;
        }
    }

    // Check against the next item
    if index < pairs.len() - 1 {
        let next_string = get_string(&pairs[index + 1]);
        // The comparison logic: `item` should be <= `next`.
        let ordering_correct = match (item_string.is_empty(), next_string.is_empty()) {
            (true, false) => false, // Empty before non-empty is INCORRECT.
            (false, true) => true,  // Non-empty before empty is correct.
            _ => item_string <= next_string,
        };
        if !ordering_correct {
            return false;
        }
    }

    // If it's sorted relative to both neighbors (or is at an edge), it's considered stable.
    true
}

fn sort_one(maybe_changed: Trigger<MaybeChanged>) {
    let MaybeChanged(i) = *maybe_changed;
    let mut pairs = PAIRS.lock_mut();

    if i >= pairs.len() {
        return;
    }

    let get_string = |p: &RowData| -> String {
        match SORT_BY.get() {
            KeyValue::Key => p.key.string.get_cloned(),
            KeyValue::Value => p.value.string.get_cloned(),
        }
    };

    // --- STAGE 1: Fast-path stability check ---
    // If the item is already in a sorted position relative to its neighbors,
    // do nothing. This completely solves the swapping loop.
    if is_sorted_at(&pairs, i, &get_string) {
        return;
    }

    // --- STAGE 2: The item is out of order, so we must find its true place and move it. ---

    // Temporarily remove the item. This simplifies the search.
    let pair_to_sort = pairs.remove(i);
    let item_string = get_string(&pair_to_sort);

    // Find the correct insertion index in the remaining (n-1) list.
    let mut insertion_index = 0;
    for other_p in pairs.iter() {
        let other_string = get_string(other_p);

        // Does `other` come before `item`?
        let other_comes_first = match (item_string.is_empty(), other_string.is_empty()) {
            (true, false) => true,  // `other` (non-empty) comes before `item` (empty).
            (false, true) => false, // `item` (non-empty) comes before `other` (empty).
            _ => other_string < item_string,
        };

        if other_comes_first {
            insertion_index += 1;
        }
    }

    // Insert the item back into the list at its correct, stable-sorted position.
    pairs.insert_cloned(insertion_index, pair_to_sort);
}

static SCROLL_POSITION: LazyLock<Mutable<f32>> = LazyLock::new(default);

fn key_values() -> impl Element + Sizeable {
    Column::<Node>::new()
        .with_node(|mut node| node.row_gap = Val::Px(10.))
        .height(Val::Percent(90.))
        .mutable_viewport(haalka::prelude::Axis::Vertical)
        .viewport_y_sync(SCROLL_POSITION.clone())
        .on_scroll_with_system_on_hover(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(20.)
                .into_system(),
        )
        .viewport_y_signal(SCROLL_POSITION.signal().dedupe())
        .items_signal_vec(PAIRS.signal_vec_cloned().enumerate().map(
            |(
                index_option,
                RowData {
                    key:
                        TextInputData {
                            string: key,
                            focus: key_focus,
                        },
                    value:
                        TextInputData {
                            string: value,
                            focus: value_focus,
                        },
                },
            )| {
                Row::<Node>::new()
                    .with_node(|mut node| node.column_gap = Val::Px(10.))
                    // without registering width up front, layout will take a frame or two to sync to size of children,
                    // making it look like the elements are expanding into place, try commenting out this line to see
                    // how it looks
                    .width(Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.))
                    .item(text_input(index_option.clone(), key, key_focus))
                    .item(text_input(index_option.clone(), value, value_focus))
                    .item(x_button().on_click(move || {
                        if let Some(index) = index_option.get() {
                            PAIRS.lock_mut().remove(index);
                        }
                    }))
            },
        ))
}

fn x_button() -> impl Element + PointerEventAware {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .apply(border_radius_style(10.))
        .width(Val::Px(INPUT_HEIGHT))
        .height(Val::Px(INPUT_HEIGHT))
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| bevy::color::palettes::basic::RED.into(), || *DARK_GRAY)
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .child(
            El::<Text>::new()
                .with_node(|mut node| node.top = Val::Px(-3.))
                .align(Align::center())
                .text_font(TextFont::from_font_size(30.))
                .text(Text::new("x")),
        )
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .ui_root()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Default))
        .child(
            Row::<Node>::new()
                .height(Val::Percent(100.))
                .with_node(|mut node| node.column_gap = Val::Px(70.))
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(20.))
                        .item(sort_button(KeyValue::Key))
                        .item(sort_button(KeyValue::Value)),
                )
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(10.))
                        .height(Val::Percent(90.))
                        .width(Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.))
                        .align_content(Align::center())
                        .item(key_values().height(Val::Percent(90.)))
                        .item({
                            let hovered = Mutable::new(false);
                            El::<Node>::new()
                                .apply(border_radius_style(10.))
                                .width(Val::Px(INPUT_WIDTH))
                                .height(Val::Px(INPUT_HEIGHT))
                                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                                .background_color_signal(
                                    hovered
                                        .signal()
                                        .map_bool(|| bevy::color::palettes::basic::GREEN.into(), || *DARK_GRAY)
                                        .map(BackgroundColor),
                                )
                                .hovered_sync(hovered)
                                .align_content(Align::center())
                                .child(
                                    El::<Text>::new()
                                        .text_font(TextFont::from_font_size(30.))
                                        .text(Text::new("+")),
                                )
                                .on_click_with_system(|_: In<_>| {
                                    clear_focus();
                                    PAIRS.lock_mut().push_cloned(RowData {
                                        key: {
                                            let data = TextInputData::new("");
                                            data.focus.set(true);
                                            data
                                        },
                                        value: TextInputData::new(""),
                                    });
                                    async {
                                        // TODO: need "after rendered" hook to exactly sync when this scroll should be
                                        // triggered
                                        sleep(Duration::from_millis(150)).await;
                                        scroll_to_bottom()
                                    }
                                    .apply(spawn)
                                    .detach();
                                })
                        }),
                ),
        )
}

fn scroll_to_bottom() {
    SCROLL_POSITION.set(f32::MAX);
}

fn tabber(keys: Res<ButtonInput<KeyCode>>) {
    // TODO: use .pressed instead of .just_pressed to allow for holding down tab, browser seems to
    // require minimum press time before starting to repeat, and repeating seems slower than refresh
    // rate
    if keys.pressed(KeyCode::ShiftLeft) && keys.just_pressed(KeyCode::Tab) {
        let pairs = PAIRS.lock_ref();
        let focused_option = pairs
            .iter()
            .position(|data| data.key.focus.get() || data.value.focus.get());
        if let Some(focused) = focused_option {
            if pairs[focused].value.focus.get() {
                pairs[focused].value.focus.set(false);
                pairs[focused].key.focus.set(true);
            } else {
                pairs[focused].key.focus.set(false);
                if focused > 0 {
                    pairs[focused - 1].value.focus.set(true);
                } else if let Some(last) = pairs.last() {
                    last.value.focus.set(true);
                }
            }
        } else if let Some(last) = pairs.last() {
            last.value.focus.set(true);
        }
    } else if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Enter) {
        let pairs = PAIRS.lock_ref();
        let focused_option = pairs
            .iter()
            .position(|data| data.key.focus.get() || data.value.focus.get());
        if let Some(focused) = focused_option {
            if pairs[focused].key.focus.get() {
                pairs[focused].key.focus.set(false);
                pairs[focused].value.focus.set(true);
            } else {
                pairs[focused].value.focus.set(false);
                if focused + 1 < pairs.len() {
                    pairs[focused + 1].key.focus.set(true);
                } else if let Some(first) = pairs.first() {
                    first.key.focus.set(true);
                }
            }
        } else if let Some(first) = pairs.first() {
            first.key.focus.set(true);
        }
    }
}

fn escaper(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        clear_focus();
    }
}

// on focus change, check if the focused element is in view, if not, scroll to it
fn focus_scroller(
    focused_text_input_option: Res<InputFocus>,
    child_ofs: Query<&ChildOf>,
    mutable_viewports: Query<&MutableViewport>,
    logical_rect: LogicalRect,
) {
    if let Some(focused_text_input) = focused_text_input_option.0
        && let Some(text_input_rect) = child_ofs.get(focused_text_input).ok().and_then(|child_of| logical_rect.get(child_of.parent()))
    {
        for ancestor in child_ofs.iter_ancestors(focused_text_input) {
            if mutable_viewports.contains(ancestor) {
                if let Some(viewport_rect) = logical_rect.get(ancestor) {
                    let d = text_input_rect.min.y - viewport_rect.min.y;
                    if d < 0. {
                        SCROLL_POSITION.update(|sp| sp + d);
                        return;
                    }
                    let d = text_input_rect.max.y - viewport_rect.max.y;
                    if d > 0. {
                        SCROLL_POSITION.update(|sp| sp + d);
                        return;
                    }
                }
                break;
            }
        }
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
