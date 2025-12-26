//! Text inputs, scrolling/viewport control, and reactive lists.
//!
//! promises made promises kept ! <https://discord.com/channels/691052431525675048/1192585689460658348/1193431789465776198>
//! (yes i take requests)

mod utils;
use bevy_input_focus::InputFocus;
use bevy_ui_text_input::{TextInputContents, TextInputMode};
use utils::*;

use std::{cmp::Ordering, collections::HashMap};

use bevy::prelude::*;
use haalka::{
    prelude::*,
    viewport_mutable::{LogicalRect, MutableViewport},
};

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(Startup, (init, camera))
        .add_systems(Update, (tabber, escaper))
        .run();
}

const INPUT_HEIGHT: f32 = 40.;
const INPUT_WIDTH: f32 = 200.;
const STARTING_SORTED_BY: KeyValue = KeyValue::Key;
const PADDING: f32 = 10.;
static DARK_GRAY: LazyLock<Color> = LazyLock::new(|| Srgba::gray(0.25).into());

#[derive(Clone, Copy, PartialEq, Eq, Hash, Resource)]
enum KeyValue {
    Key,
    Value,
}

#[derive(Component, Clone, Deref, DerefMut)]
struct KeyString(String);

impl From<String> for KeyString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Component, Clone, Deref, DerefMut)]
struct ValueString(String);

impl From<String> for ValueString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Component, Clone, Copy)]
struct ModelEntityRef(Entity);

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
struct InputField(KeyValue);

#[derive(Component, Clone, Copy)]
struct FocusOnSpawn(KeyValue);

#[derive(Resource, Clone, Deref, DerefMut)]
struct Pairs(MutableVec<Entity>);

const ROW_GAP: f32 = 10.;
const ROW_STEP: f32 = INPUT_HEIGHT + ROW_GAP; // 50px per row

// fn clear_focus(foci: &mut Query<&mut Focused>) {
//     for mut focus in foci.iter_mut() {
//         *focus = Focused(false);
//     }
// }

fn sort_by_text_element() -> El<Text> {
    El::<Text>::new()
        .text_font(TextFont::from_font_size(60.))
        .text_color(TextColor(Color::WHITE))
        .text(Text::new("sort by"))
}

fn init(world: &mut World) {
    world.insert_resource(STARTING_SORTED_BY);

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

    pairs.sort_by(|(a_key, a_value), (b_key, b_value)| match STARTING_SORTED_BY {
        KeyValue::Key => cmp_text(a_key, b_key),
        KeyValue::Value => cmp_text(a_value, b_value),
    });

    let model_entities = pairs
        .into_iter()
        .map(|(key, value)| {
            world
                .spawn((KeyString(key.to_string()), ValueString(value.to_string())))
                .id()
        })
        .collect::<Vec<_>>();

    let pairs = Pairs(MutableVecBuilder::from(model_entities).spawn(world));
    world.insert_resource(pairs.clone());

    ui_root(pairs).spawn(world);
}

fn text_input(model_entity: Entity, field: KeyValue, initial: String) -> impl Element {
    let ui_entity = LazyEntity::new();
    let focused = SignalBuilder::from_resource::<InputFocus>()
        .map_in(clone!((ui_entity) move |focus| focus.0 == Some(*ui_entity)))
        .dedupe();

    El::<Node>::new()
        .insert(BorderRadius::all(Val::Px(10.)))
        .insert(Pickable::default())
        .with_node(|mut node| {
            node.height = Val::Px(INPUT_HEIGHT);
            node.width = Val::Px(INPUT_WIDTH);
            node.overflow = Overflow::clip();
        })
        .background_color_signal(
            focused
                .clone()
                .map_bool_in(|| Color::WHITE, || *DARK_GRAY)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .cursor(CursorIcon::System(SystemCursorIcon::Text))
        .on_click(clone!((ui_entity) move |In(_), mut input_focus: ResMut<InputFocus>| {
            input_focus.0 = Some(*ui_entity);
        }))
        .on_click_outside(clone!((ui_entity) move |In(_), mut input_focus: ResMut<InputFocus>| {
            if input_focus.0 == Some(*ui_entity) {
                input_focus.0 = None;
            }
        }))
        .child(
            TextInput::new()
                .lazy_entity(ui_entity.clone())
                .insert((
                    TextInputContents::default(),
                    ModelEntityRef(model_entity),
                    InputField(field),
                ))
                .text(initial)
                .align(Align::new().center_y())
                .with_node(|mut node| {
                    node.left = Val::Px(PADDING);
                    node.width = Val::Px(INPUT_WIDTH - PADDING * 2.);
                    node.height = Val::Px(INPUT_HEIGHT - PADDING * 2. + 5.);
                })
                .with_text_input_node(|mut node| {
                    node.mode = TextInputMode::SingleLine;
                    // TODO: https://github.com/ickshonpe/bevy_ui_text_input/issues/10
                    // node.justification = JustifyText::Center;
                })
                .with_builder(move |builder| {
                    builder.on_spawn(move |world: &mut World, entity: Entity| {
                        if world
                            .get::<FocusOnSpawn>(model_entity)
                            .is_some_and(|focus| focus.0 == field)
                        {
                            world.resource_mut::<InputFocus>().0 = Some(entity);
                            world.entity_mut(model_entity).remove::<FocusOnSpawn>();
                        }
                    })
                })
                .text_color_signal(
                    focused
                        .clone()
                        .map_bool_in(|| Color::BLACK, || Color::WHITE)
                        .map_in(TextColor)
                        .map_in(Some),
                )
                // On focus change, check if the focused element is in view, if not, scroll to it
                .on_focused_change(
                    |In((entity, is_focused)): In<(Entity, bool)>,
                     child_ofs: Query<&ChildOf>,
                     mutable_viewports: Query<Entity, With<MutableViewport>>,
                     mut scroll_positions: Query<&mut ScrollPosition>,
                     logical_rect: LogicalRect| {
                        if !is_focused {
                            return;
                        }
                        let Some(text_input_rect) = child_ofs
                            .get(entity)
                            .ok()
                            .and_then(|child_of| logical_rect.get(child_of.parent()))
                        else {
                            return;
                        };
                        for ancestor in child_ofs.iter_ancestors(entity) {
                            if mutable_viewports.contains(ancestor) {
                                let Some(viewport_rect) = logical_rect.get(ancestor) else {
                                    return;
                                };
                                let d = text_input_rect.min.y - viewport_rect.min.y;
                                if d < 0. {
                                    if let Ok(mut sp) = scroll_positions.get_mut(ancestor) {
                                        sp.0.y += d;
                                    }
                                    return;
                                }
                                let d = text_input_rect.max.y - viewport_rect.max.y;
                                if d > 0. {
                                    if let Ok(mut sp) = scroll_positions.get_mut(ancestor) {
                                        sp.0.y += d;
                                    }
                                    return;
                                }
                                return;
                            }
                        }
                    },
                )
                .on_change(
                    move |In((ui, text)): In<(Entity, String)>,
                          sort_by: Res<KeyValue>,
                          pairs: Res<Pairs>,
                          input_focus: Res<InputFocus>,
                          mut commands: Commands,
                          model_refs: Query<&ModelEntityRef>,
                          fields: Query<&InputField>,
                          key_strings: Query<&KeyString>,
                          value_strings: Query<&ValueString>,
                          mut scroll_position: Single<&mut bevy::ui::ScrollPosition, With<MutableViewport>>,
                          mut vec_datas: Query<&mut MutableVecData<Entity>>| {
                        let Ok(model_entity_ref) = model_refs.get(ui) else {
                            return;
                        };
                        let model_entity = model_entity_ref.0;

                        match field {
                            KeyValue::Key => {
                                commands.entity(model_entity).insert(KeyString(text.clone()));
                            }
                            KeyValue::Value => {
                                commands.entity(model_entity).insert(ValueString(text.clone()));
                            }
                        }

                        // Live reordering only affects the current sort field.
                        if fields.get(ui).ok().map(|f| f.0) != Some(*sort_by) {
                            return;
                        }

                        let mut write = pairs.0.write(&mut vec_datas);
                        let items = write.to_vec();
                        let Some(old_index) = items.iter().position(|e| *e == model_entity) else {
                            return;
                        };

                        let moved_str: &str = text.as_str();

                        let mut new_index = 0usize;
                        for (i, other) in items.iter().enumerate() {
                            if *other == model_entity {
                                continue;
                            }
                            let other_str: &str = match *sort_by {
                                KeyValue::Key => key_strings.get(*other).map(|s| s.as_str()).unwrap_or(""),
                                KeyValue::Value => value_strings.get(*other).map(|s| s.as_str()).unwrap_or(""),
                            };
                            let cmp = cmp_text(other_str, moved_str);
                            // For stable sorting: count items that are Less, or Equal but originally before us
                            if cmp == Ordering::Less || (cmp == Ordering::Equal && i < old_index) {
                                new_index += 1;
                            }
                        }

                        if new_index != old_index {
                            write.move_item(old_index, new_index);

                            // Adjust scroll to keep focused input visually stationary
                            if input_focus.0 == Some(ui) {
                                let delta = (new_index as f32 - old_index as f32) * ROW_STEP;
                                scroll_position.0.y += delta;
                            }
                        }
                    },
                ),
        )
}

fn sort_button(sort_by: KeyValue) -> impl Element {
    let lazy_entity = LazyEntity::new();
    let selected = SignalBuilder::from_resource::<KeyValue>().dedupe().eq(sort_by).dedupe();
    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(35.))
        .align(Align::new().right())
        .item_signal(selected.clone().map_true_in(sort_by_text_element))
        .item(
            El::<Node>::new()
                .insert(BorderRadius::all(Val::Px(20.)))
                .insert(Pickable::default())
                .with_node(|mut node| {
                    node.width = Val::Px(200.);
                    node.height = Val::Px(80.);
                })
                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                .lazy_entity(lazy_entity.clone())
                .background_color_signal(
                    signal::any!(
                        SignalBuilder::from_lazy_entity(lazy_entity)
                            .has_component::<Hovered>()
                            .dedupe(),
                        selected.clone()
                    )
                    .dedupe()
                    .map_bool_in(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
                    .map_in(BackgroundColor)
                    .map_in(Some),
                )
                .align_content(Align::center())
                .on_click(
                    move |_: In<_>,
                          mut current_sort: ResMut<KeyValue>,
                          pairs: Res<Pairs>,
                          key_strings: Query<&KeyString>,
                          value_strings: Query<&ValueString>,
                          mut vec_datas: Query<&mut MutableVecData<Entity>>| {
                        if *current_sort == sort_by {
                            return;
                        }
                        *current_sort = sort_by;

                        let mut write = pairs.0.write(&mut vec_datas);
                        let mut current = write.to_vec();
                        let mut desired = current.clone();

                        desired.sort_by(|a, b| {
                            let a_str: &str = match sort_by {
                                KeyValue::Key => key_strings.get(*a).map(|s| s.as_str()).unwrap_or(""),
                                KeyValue::Value => value_strings.get(*a).map(|s| s.as_str()).unwrap_or(""),
                            };
                            let b_str: &str = match sort_by {
                                KeyValue::Key => key_strings.get(*b).map(|s| s.as_str()).unwrap_or(""),
                                KeyValue::Value => value_strings.get(*b).map(|s| s.as_str()).unwrap_or(""),
                            };
                            cmp_text(a_str, b_str)
                        });

                        for target_index in 0..desired.len() {
                            if current[target_index] == desired[target_index] {
                                continue;
                            }
                            let Some(from_index) = current.iter().position(|e| *e == desired[target_index]) else {
                                continue;
                            };
                            current.remove(from_index);
                            current.insert(target_index, desired[target_index]);
                            write.move_item(from_index, target_index);
                        }
                    },
                )
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

fn cmp_text(a: &str, b: &str) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => a.cmp(b),
    }
}

// ----- Viewport sizing hierarchy -----
// The viewport (scrollable key_values Column) is nested:
//   Window (100vh)
//     └─ outer Column (90% of window)
//         └─ key_values Column (90% of outer) ← this is the viewport
//
// Therefore: viewport height = 90% × 90% = 81% of window height (81vh)
const OUTER_COLUMN_HEIGHT_PERCENT: f32 = 90.;
const KEY_VALUES_HEIGHT_PERCENT: f32 = 90.;
const VIEWPORT_HEIGHT_VH: f32 = OUTER_COLUMN_HEIGHT_PERCENT * KEY_VALUES_HEIGHT_PERCENT / 100.;

/// Empty element matching viewport height, allowing scroll past content bounds for focus anchoring.
fn viewport_spacer() -> El<Node> {
    El::<Node>::new().with_node(|mut node| {
        node.min_height = Val::Vh(VIEWPORT_HEIGHT_VH);
    })
}

fn key_values(pairs: Pairs) -> Column<Node> {
    Column::<Node>::new()
        .with_node(|mut node| {
            node.row_gap = Val::Px(ROW_GAP);
            node.height = Val::Percent(KEY_VALUES_HEIGHT_PERCENT);
        })
        .insert(Pickable::default())
        .mutable_viewport(Overflow::scroll_y())
        .on_scroll(BasicScrollHandler::new().pixels(20.).into_system())
        .with_builder(|builder| {
            builder.on_spawn(|world: &mut World, entity: Entity| {
                // Set initial scroll to skip past the top spacer
                let Ok(window) = world.query::<&Window>().single(world) else {
                    return;
                };
                let spacer_px = window.height() * VIEWPORT_HEIGHT_VH / 100.;
                world
                    .entity_mut(entity)
                    .insert(ScrollPosition(Vec2::new(0., spacer_px + ROW_GAP)));
            })
        })
        .item(viewport_spacer())
        .items_signal_vec(pairs.0.signal_vec().enumerate().map(
            |In((index_signal, model_entity)): In<(signal::Source<Option<usize>>, Entity)>,
             key_strings: Query<&KeyString>,
             value_strings: Query<&ValueString>| {
                let key_initial = key_strings
                    .get(model_entity)
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default();
                let value_initial = value_strings
                    .get(model_entity)
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default();

                Row::<Node>::new()
                    .with_node(|mut node| {
                        node.column_gap = Val::Px(10.);
                        node.width = Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.)
                    })
                    .insert(Pickable::default())
                    .item(text_input(model_entity, KeyValue::Key, key_initial))
                    .item(text_input(model_entity, KeyValue::Value, value_initial))
                    .item(
                        x_button()
                            .with_builder(|builder| builder.component_signal(index_signal.map_some_in(Index)))
                            .on_click(
                                move |In((entity, _)): In<_>,
                                      pairs: Res<Pairs>,
                                      indices: Query<&Index>,
                                      mut commands: Commands,
                                      mut vec_datas: Query<&mut MutableVecData<Entity>>| {
                                    let index = indices.get(entity).unwrap().0;
                                    let removed = pairs.0.write(&mut vec_datas).remove(index);
                                    commands.entity(removed).despawn();
                                },
                            ),
                    )
            },
        ))
        .item(viewport_spacer())
}

#[derive(Component, Clone)]
struct Index(usize);

fn x_button() -> impl Element + PointerEventAware {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .insert(BorderRadius::all(Val::Px(10.)))
        .insert(Pickable::default())
        .with_node(|mut node| {
            node.width = Val::Px(INPUT_HEIGHT);
            node.height = Val::Px(INPUT_HEIGHT);
        })
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .lazy_entity(lazy_entity.clone())
        .background_color_signal(
            SignalBuilder::from_lazy_entity(lazy_entity)
                .has_component::<Hovered>()
                .dedupe()
                .map_bool_in(|| bevy::color::palettes::basic::RED.into(), || *DARK_GRAY)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .child(
            El::<Text>::new()
                .with_node(|mut node| node.top = Val::Px(-3.))
                .align(Align::center())
                .text_font(TextFont::from_font_size(30.))
                .text(Text::new("x")),
        )
}

fn ui_root(pairs: Pairs) -> impl Element {
    El::<Node>::new()
        .ui_root()
        .insert(Pickable::default())
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .cursor(CursorIcon::default())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| {
                    node.height = Val::Percent(100.);
                    node.column_gap = Val::Px(70.);
                })
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(20.))
                        .item(sort_button(KeyValue::Key))
                        .item(sort_button(KeyValue::Value)),
                )
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| {
                            node.row_gap = Val::Px(10.);
                            node.height = Val::Percent(OUTER_COLUMN_HEIGHT_PERCENT);
                            node.width = Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.);
                        })
                        .align_content(Align::center())
                        .item(
                            key_values(pairs.clone())
                                .with_node(|mut node| node.height = Val::Percent(KEY_VALUES_HEIGHT_PERCENT)),
                        )
                        .item({
                            let lazy_entity = LazyEntity::new();
                            El::<Node>::new()
                                .insert(BorderRadius::all(Val::Px(10.)))
                                .insert(Pickable::default())
                                .with_node(|mut node| {
                                    node.width = Val::Px(INPUT_WIDTH);
                                    node.height = Val::Px(INPUT_HEIGHT);
                                })
                                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                                .lazy_entity(lazy_entity.clone())
                                .background_color_signal(
                                    SignalBuilder::from_lazy_entity(lazy_entity)
                                        .has_component::<Hovered>()
                                        .dedupe()
                                        .map_bool_in(|| bevy::color::palettes::basic::GREEN.into(), || *DARK_GRAY)
                                        .map_in(BackgroundColor)
                                        .map_in(Some),
                                )
                                .align_content(Align::center())
                                .child(
                                    El::<Text>::new()
                                        .text_font(TextFont::from_font_size(30.))
                                        .text(Text::new("+")),
                                )
                                .on_click(
                                    |In((entity, click)): In<(Entity, Pointer<Click>)>,
                                     pairs: Res<Pairs>,
                                     mut commands: Commands,
                                     mut scroll_positions: Query<
                                        &mut bevy::ui::ScrollPosition,
                                        With<MutableViewport>,
                                    >,
                                     mut vec_datas: Query<&mut MutableVecData<Entity>>| {
                                        info!("Plus button clicked: entity={:?}, hit={:?}", entity, click.hit);
                                        let model_entity = commands
                                            .spawn((
                                                KeyString(String::new()),
                                                ValueString(String::new()),
                                                FocusOnSpawn(KeyValue::Key),
                                            ))
                                            .id();
                                        let mut write = pairs.0.write(&mut vec_datas);
                                        write.push(model_entity);
                                        let count = write.to_vec().len();
                                        drop(write);
                                        // Scroll so the new row's bottom aligns with viewport bottom
                                        for mut scroll_position in scroll_positions.iter_mut() {
                                            scroll_position.0.y = count as f32 * ROW_STEP;
                                        }
                                    },
                                )
                        }),
                ),
        )
}

fn tabber(
    keys: Res<ButtonInput<KeyCode>>,
    pairs: Res<Pairs>,
    vec_datas: Query<&MutableVecData<Entity>>,
    inputs: Query<(Entity, &ModelEntityRef, &InputField)>,
    model_refs: Query<&ModelEntityRef>,
    fields: Query<&InputField>,
    mut focused_option: ResMut<InputFocus>,
) {
    let rows = pairs.0.read(&vec_datas);

    let mut by_model_and_field: HashMap<(Entity, KeyValue), Entity> = HashMap::new();
    for (ui, model, field) in inputs.iter() {
        by_model_and_field.insert((model.0, field.0), ui);
    }

    let focused_data = (|| {
        let focused = focused_option.0?;
        let model_entity = model_refs.get(focused).ok()?.0;
        let field = fields.get(focused).ok()?.0;
        let index = rows.iter().position(|e| *e == model_entity)?;
        Some((index, field))
    })();

    let mut set_focus = |model_entity: Entity, field: KeyValue| {
        if let Some(ui) = by_model_and_field.get(&(model_entity, field)).copied() {
            focused_option.0 = Some(ui);
        }
    };

    // TODO: use .pressed instead of .just_pressed to allow for holding down tab, browser seems to
    // require minimum press time before starting to repeat, and repeating seems slower than refresh
    // rate
    if keys.pressed(KeyCode::ShiftLeft) && keys.just_pressed(KeyCode::Tab) {
        match focused_data {
            None => {
                if let Some(last_model) = rows.last().copied() {
                    set_focus(last_model, KeyValue::Value);
                }
            }
            Some((i, KeyValue::Value)) => {
                set_focus(rows[i], KeyValue::Key);
            }
            Some((i, KeyValue::Key)) => {
                let prev = if i > 0 { i - 1 } else { rows.len().saturating_sub(1) };
                if let Some(prev_model) = rows.get(prev).copied() {
                    set_focus(prev_model, KeyValue::Value);
                }
            }
        }
    } else if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Enter) {
        match focused_data {
            None => {
                if let Some(first_model) = rows.first().copied() {
                    set_focus(first_model, KeyValue::Key);
                }
            }
            Some((i, KeyValue::Key)) => {
                set_focus(rows[i], KeyValue::Value);
            }
            Some((i, KeyValue::Value)) => {
                let next = if i + 1 < rows.len() { i + 1 } else { 0 };
                if let Some(next_model) = rows.get(next).copied() {
                    set_focus(next_model, KeyValue::Key);
                }
            }
        }
    }
}

fn escaper(keys: Res<ButtonInput<KeyCode>>, mut input_focus: ResMut<InputFocus>) {
    if keys.just_pressed(KeyCode::Escape) {
        input_focus.0 = None;
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
