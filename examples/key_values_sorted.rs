//! Text inputs, scrolling/viewport control, and reactive lists.
//!
//! promises made promises kept ! <https://discord.com/channels/691052431525675048/1192585689460658348/1193431789465776198>
//! (yes i take requests)

mod utils;
use utils::*;

use std::{
    ops::{Deref, Not},
    time::Duration,
};

use bevy::prelude::*;
use bevy_cosmic_edit::{CosmicBackgroundColor, CosmicWrap, CursorColor, MaxLines};
use haalka::{prelude::*, text_input::FocusedTextInput, viewport_mutable::MutableViewport};

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
                sort_one.run_if(on_event::<MaybeChanged>()),
                focus_scroller.run_if(resource_changed_or_removed::<FocusedTextInput>()),
            ),
        )
        .add_event::<MaybeChanged>()
        .run();
}

const INPUT_HEIGHT: f32 = 40.;
const INPUT_WIDTH: f32 = 200.;
const STARTING_SORTED_BY: KeyValue = KeyValue::Key;
static DARK_GRAY: Lazy<Color> = Lazy::new(|| Srgba::gray(0.25).into());

static PAIRS: Lazy<MutableVec<RowData>> = Lazy::new(|| {
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

static SORT_BY: Lazy<Mutable<KeyValue>> = Lazy::new(|| Mutable::new(STARTING_SORTED_BY));

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
    TextInput::new()
        .width(Val::Px(INPUT_WIDTH))
        .height(Val::Px(INPUT_HEIGHT))
        .mode(CosmicWrap::InfiniteLine)
        .max_lines(MaxLines(1))
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
                .map_bool(|| *DARK_GRAY, || Color::WHITE)
                .map(CosmicBackgroundColor),
        )
        .attrs(TextAttrs::new().color_signal(focus.signal().map_bool(|| Color::WHITE, || Color::BLACK).map(Some)))
        .focus_signal(focus.signal())
        .on_focused_change(clone!((focus) move |is_focused| {
            if !is_focused {
                if let Some(index) = index_option.get() {
                    // TODO: use an observer for this
                    async_world().send_event(MaybeChanged(index)).apply(spawn).detach()
                }
            }
            focus.set_neq(is_focused);
        }))
        .text_signal(string.signal_cloned())
        .on_change_sync(string)
    // TODO: this unfocuses on click for some reason ...
    // .on_click_outside_with_system(|In(_), mut commands: Commands|
    // commands.remove_resource::<FocusedTextInput>())
}

fn clear_focus() {
    for RowData { key, value } in PAIRS.lock_ref().iter() {
        key.focus.set_neq(false);
        value.focus.set_neq(false);
    }
}

fn sort_by_text_element() -> impl Element {
    El::<TextBundle>::new().text(Text::from_section(
        "sort by",
        TextStyle {
            font_size: 60.,
            color: Color::WHITE,
            ..default()
        },
    ))
}

fn sort_button(sort_by: KeyValue) -> impl Element {
    let hovered = Mutable::new(false);
    let selected = SORT_BY.signal().map(move |cur| cur == sort_by).broadcast();
    Row::<NodeBundle>::new()
        .with_style(|mut style| style.column_gap = Val::Px(35.))
        .align(Align::new().right())
        .item_signal(selected.signal().map_true(sort_by_text_element))
        .item(
            El::<NodeBundle>::new()
                .width(Val::Px(200.))
                .height(Val::Px(80.))
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
                .child(El::<TextBundle>::new().text(Text::from_section(
                    match sort_by {
                        KeyValue::Key => "key",
                        KeyValue::Value => "value",
                    },
                    TextStyle {
                        font_size: 60.,
                        color: Color::WHITE,
                        ..default()
                    },
                ))),
        )
}

#[derive(Clone, Copy, Event)]
struct MaybeChanged(usize);

// O(log n)
fn sort_one(mut maybe_changed_events: EventReader<MaybeChanged>) {
    for MaybeChanged(i) in maybe_changed_events.read().copied() {
        let mut pairs = PAIRS.lock_mut();
        let Some(RowData { key, value }) = pairs.get(i) else {
            return;
        };
        match SORT_BY.get() {
            // TODO: dry
            KeyValue::Key => {
                let keys = pairs
                    .iter()
                    .enumerate()
                    .filter(|&(j, value)| i != j && value.key.string.lock_ref().is_empty().not())
                    .map(|(_, RowData { key, .. })| key.string.lock_ref())
                    .collect::<Vec<_>>();
                let key_lock = key.string.lock_ref();
                if key_lock.is_empty().not() {
                    let (Ok(sorted_i) | Err(sorted_i)) =
                        keys.binary_search_by_key(&key_lock.as_str(), |key| key.as_str());
                    if i != sorted_i && key_lock.as_str() != keys[sorted_i].as_str() {
                        drop((keys, key_lock));
                        let pair = pairs.remove(i);
                        pairs.insert_cloned(sorted_i, pair);
                    }
                }
            }
            KeyValue::Value => {
                let values = pairs
                    .iter()
                    .enumerate()
                    .filter(|&(j, value)| i != j && value.value.string.lock_ref().is_empty().not())
                    .map(|(_, RowData { value, .. })| value.string.lock_ref())
                    .collect::<Vec<_>>();
                let value_lock = value.string.lock_ref();
                if value_lock.is_empty().not() {
                    let (Ok(sorted_i) | Err(sorted_i)) =
                        values.binary_search_by_key(&value_lock.as_str(), |value| value.as_str());
                    if i != sorted_i && value_lock.as_str() != values[sorted_i].as_str() {
                        drop((values, value_lock));
                        let pair = pairs.remove(i);
                        pairs.insert_cloned(sorted_i, pair);
                    }
                }
            }
        }
    }
}

static SCROLL_POSITION: Lazy<Mutable<f32>> = Lazy::new(default);

fn key_values() -> impl Element + Sizeable {
    Column::<NodeBundle>::new()
        .with_style(|mut style| style.row_gap = Val::Px(10.))
        .height(Val::Percent(90.))
        .mutable_viewport(Overflow::clip_y(), LimitToBody::Vertical)
        .on_scroll_with_system_on_hover(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(20.)
                .into_system(),
        )
        .viewport_y_signal(SCROLL_POSITION.signal())
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
                Row::<NodeBundle>::new()
                    .with_style(|mut style| style.column_gap = Val::Px(10.))
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
    El::<NodeBundle>::new()
        .width(Val::Px(INPUT_HEIGHT))
        .height(Val::Px(INPUT_HEIGHT))
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| bevy::color::palettes::basic::RED.into(), || *DARK_GRAY)
                .map(BackgroundColor::from),
        )
        .hovered_sync(hovered)
        .child(
            El::<TextBundle>::new()
                .with_style(|mut style| style.top = Val::Px(-3.))
                .align(Align::center())
                .text(Text::from_section(
                    "x",
                    TextStyle {
                        font_size: 30.0,
                        ..default()
                    },
                )),
        )
}

fn ui_root() -> impl Element {
    El::<NodeBundle>::new()
        .ui_root()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<NodeBundle>::new()
                .height(Val::Percent(100.))
                .with_style(|mut style| style.column_gap = Val::Px(70.))
                .item(
                    Column::<NodeBundle>::new()
                        .with_style(|mut style| style.row_gap = Val::Px(20.))
                        .item(sort_button(KeyValue::Key))
                        .item(sort_button(KeyValue::Value)),
                )
                .item(
                    Column::<NodeBundle>::new()
                        .with_style(|mut style| style.row_gap = Val::Px(10.))
                        .height(Val::Percent(90.))
                        .width(Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.))
                        .align_content(Align::center())
                        .item(key_values().height(Val::Percent(90.)))
                        .item({
                            let hovered = Mutable::new(false);
                            El::<NodeBundle>::new()
                                .width(Val::Px(INPUT_WIDTH))
                                .height(Val::Px(INPUT_HEIGHT))
                                .background_color_signal(
                                    hovered
                                        .signal()
                                        .map_bool(|| bevy::color::palettes::basic::GREEN.into(), || *DARK_GRAY)
                                        .map(BackgroundColor::from),
                                )
                                .hovered_sync(hovered)
                                .align_content(Align::center())
                                .child(El::<TextBundle>::new().text(Text::from_section(
                                    "+",
                                    TextStyle {
                                        font_size: 30.0,
                                        ..default()
                                    },
                                )))
                                .on_click_with_system(|_: In<_>, mut commands: Commands| {
                                    commands.remove_resource::<FocusedTextInput>(); // TODO: shouldn't need this, can remove once https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
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
                                        sleep(Duration::from_millis(25)).await;
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
    SCROLL_POSITION.set(f32::MIN);
}

fn tabber(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    // TODO: use .pressed instead of .just_pressed to allow for holding down tab, browser seems to
    // require minimum press time before starting to repeat, and repeating seems slower than refresh
    // rate
    if keys.pressed(KeyCode::ShiftLeft) && keys.just_pressed(KeyCode::Tab) {
        commands.remove_resource::<FocusedTextInput>(); // TODO: shouldn't need this, but text color doesn't sync otherwise https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
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
        } else {
            if let Some(last) = pairs.last() {
                last.value.focus.set(true);
            }
        }
    } else if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Enter) {
        commands.remove_resource::<FocusedTextInput>(); // TODO: shouldn't need this, but text color doesn't sync otherwise https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
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
        } else {
            if let Some(first) = pairs.first() {
                first.key.focus.set(true);
            }
        }
    }
}

fn escaper(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<FocusedTextInput>(); // TODO: shouldn't need this, but text color doesn't sync otherwise https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
        clear_focus();
    }
}

// on focus change, check if the focused element is in view, if not, scroll to it
fn focus_scroller(
    focused_text_input_option: Option<Res<FocusedTextInput>>,
    data_query: Query<(&Node, &GlobalTransform, &mut Style)>,
    parents: Query<&Parent>,
    mutable_viewports: Query<&MutableViewport>,
) {
    if let Some(focused_text_input) = focused_text_input_option.as_deref().map(Deref::deref).copied() {
        if let Ok((text_input_node, text_input_transform, _)) = data_query.get(focused_text_input) {
            for parent in parents.iter_ancestors(focused_text_input) {
                if mutable_viewports.contains(parent) {
                    if let Ok((scene_node, scene_transform, scene_style)) = data_query.get(parent) {
                        if let Some((viewport_node, viewport_transform, viewport_style)) = parents
                            .get(parent)
                            .ok()
                            .and_then(|parent| data_query.get(parent.get()).ok())
                        {
                            let text_input_rect = text_input_node.logical_rect(text_input_transform);
                            let scene_rect = scene_node.logical_rect(scene_transform);
                            let viewport_rect = viewport_node.logical_rect(viewport_transform);
                            let scrolled_option = match viewport_style.top {
                                Val::Px(top) => Some(top),
                                Val::Auto => Some(0.0),
                                _ => None,
                            };
                            if let Some(scrolled) = scrolled_option {
                                let container_base = viewport_rect.min.y - scrolled;
                                let child_offset = text_input_rect.min.y - scrolled - container_base;
                                // TODO: is there a simpler/ more general way to check for node visibility ?
                                if child_offset + INPUT_HEIGHT - scrolled > viewport_rect.height() {
                                    SCROLL_POSITION.set(
                                        scene_rect.min.y - text_input_rect.min.y + viewport_rect.height()
                                            - INPUT_HEIGHT,
                                    );
                                } else if child_offset < scrolled {
                                    SCROLL_POSITION.set(scene_rect.min.y - text_input_rect.min.y);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
