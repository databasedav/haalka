//! Text inputs, scrolling/viewport control, and reactive lists.
//!
//! promises made promises kept ! <https://discord.com/channels/691052431525675048/1192585689460658348/1193431789465776198>
//! (yes i take requests)

mod utils;
use bevy_input_focus::InputFocus;
use bevy_ui_text_input::TextInputMode;
use utils::*;

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
                |world: &mut World| initialize(world),
                camera,
            ),
        )
        .add_systems(Update, (tabber, escaper, autofocus))
        .run();
}

const INPUT_HEIGHT: f32 = 40.;
const INPUT_WIDTH: f32 = 200.;
const STARTING_SORTED_BY: KeyValue = KeyValue::Key;
const PADDING: f32 = 10.;
static DARK_GRAY: LazyLock<Color> = LazyLock::new(|| Srgba::gray(0.25).into());

#[derive(Clone, Copy, PartialEq)]
enum KeyValue {
    Key,
    Value,
}

#[derive(Resource, Clone, Copy)]
struct SortBy(KeyValue);

#[derive(Resource, Clone)]
struct Pairs(MutableVec<RowData>);

#[derive(Resource, Default)]
struct NextRowId(u64);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct RowId(u64);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Key,
    Value,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct RowField {
    row_id: RowId,
    field: Field,
}

#[derive(Component)]
struct AutoFocus;

#[derive(Clone)]
struct RowData {
    id: RowId,
    key: String,
    value: String,
    autofocus_key: bool,
}

fn initialize(world: &mut World) {
    world.insert_resource(SortBy(STARTING_SORTED_BY));
    world.insert_resource(NextRowId(0));

    let mut initial_pairs = [
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
        KeyValue::Key => initial_pairs.sort_by_key(|&(k, _)| k),
        KeyValue::Value => initial_pairs.sort_by_key(|&(_, v)| v),
    }

    let mut next_id = world.resource::<NextRowId>().0;
    let initial_values = initial_pairs
        .into_iter()
        .map(|(key, value)| {
            let id = RowId(next_id);
            next_id += 1;
            RowData {
                id,
                key: key.to_string(),
                value: value.to_string(),
                autofocus_key: false,
            }
        })
        .collect::<Vec<_>>();
    world.resource_mut::<NextRowId>().0 = next_id;

    let pairs = MutableVecBuilder::from(initial_values).spawn(world);
    world.insert_resource(Pairs(pairs.clone()));

    let viewport_holder = LazyEntity::new();
    ui_root(pairs, viewport_holder).spawn(world);
}

fn text_input(
    pairs: MutableVec<RowData>,
    row_id: RowId,
    field: Field,
    initial_text: String,
    autofocus: bool,
) -> impl Element {
    let text_input_holder = LazyEntity::new();

    // Focused if the InputFocus resource points at this TextInput entity.
    let focused_signal = SignalBuilder::from_system(clone!((text_input_holder) move |_: In<()>, focus: Option<Res<InputFocus>>| {
        Some(
            focus
                .as_deref()
                .map(|f| f.0 == Some(text_input_holder.get()))
                .unwrap_or(false),
        )
    }))
    .dedupe();

    El::<Node>::new()
        .apply(border_radius_style(10.))
        .with_node(|mut node| {
            node.height = Val::Px(INPUT_HEIGHT);
            node.width = Val::Px(INPUT_WIDTH);
            node.overflow = Overflow::clip();
        })
        .background_color_signal(
            focused_signal
                .clone()
                .map_bool_in(|| Color::WHITE, || *DARK_GRAY)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .cursor(CursorIcon::System(SystemCursorIcon::Text))
        .on_click(clone!((text_input_holder) move |_: In<_>, mut input_focus: ResMut<InputFocus>| {
            input_focus.0 = Some(text_input_holder.get());
        }))
        .on_click_outside(clone!((text_input_holder) move |_: In<_>, mut input_focus: ResMut<InputFocus>| {
            if input_focus.0 == Some(text_input_holder.get()) {
                input_focus.0 = None;
            }
        }))
        .child(
            TextInput::new()
                .align(Align::new().center_y())
                .with_builder(|builder| {
                    let builder = builder
                        .lazy_entity(text_input_holder.clone())
                        .insert(RowField { row_id, field });
                    if autofocus {
                        builder.insert(AutoFocus)
                    } else {
                        builder
                    }
                })
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
                .text_color_signal(
                    focused_signal
                        .clone()
                        .map_bool_in(|| Color::BLACK, || Color::WHITE)
                        .map_in(TextColor)
                        .map_in(Some),
                )
                .focus_signal(focused_signal.clone())
                .text(initial_text)
                .on_focused_change_with_system(clone!((pairs) move |In((entity, is_focused)): In<(Entity, bool)>, contents: Query<&bevy_ui_text_input::TextInputContents>, sort_by: Res<SortBy>, mut pairs_data: Query<&mut MutableVecData<RowData>>| {
                    if !is_focused {
                        if let Ok(contents) = contents.get(entity) {
                            let text = contents.get().to_string();
                            {
                                let mut guard = pairs.write(&mut pairs_data);
                                for i in 0..guard.len() {
                                    if guard[i].id == row_id {
                                        let mut updated = guard[i].clone();
                                        match field {
                                            Field::Key => updated.key = text.clone(),
                                            Field::Value => updated.value = text.clone(),
                                        }
                                        updated.autofocus_key = false;
                                        guard.set(i, updated);
                                        break;
                                    }
                                }
                            }
                            reorder_pairs(sort_by.0, &pairs, &mut pairs_data);
                        }
                    }
                }))
                // on focus change, check if the focused element is in view, if not, scroll to it
                .on_focused_change_with_system(
                    |In((entity, is_focused)),
                     child_ofs: Query<&ChildOf>,
                     mutable_viewports: Query<&MutableViewport>,
                     mut scroll_positions: Query<&mut ScrollPosition>,
                     logical_rect: LogicalRect| {
                        if is_focused
                            && let Some(text_input_rect) = child_ofs
                                .get(entity)
                                .ok()
                                .and_then(|child_of| logical_rect.get(child_of.parent()))
                        {
                            for ancestor in child_ofs.iter_ancestors(entity) {
                                if mutable_viewports.contains(ancestor) {
                                    if let Some(viewport_rect) = logical_rect.get(ancestor) {
                                        let d = text_input_rect.min.y - viewport_rect.min.y;
                                        if d < 0. {
                                            if let Ok(mut sp) = scroll_positions.get_mut(ancestor) {
                                                sp.y += d;
                                            }
                                            return;
                                        }
                                        let d = text_input_rect.max.y - viewport_rect.max.y;
                                        if d > 0. {
                                            if let Ok(mut sp) = scroll_positions.get_mut(ancestor) {
                                                sp.y += d;
                                            }
                                            return;
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    },
                )
        )
}

fn sort_by_text_element() -> El<Text> {
    El::<Text>::new()
        .text_font(TextFont::from_font_size(60.))
        .text_color(TextColor(Color::WHITE))
        .text(Text::new("sort by"))
}

fn border_radius_style(border_radius: f32) -> impl FnOnce(El<Node>) -> El<Node> {
    move |el| el.border_radius(BorderRadius::all(Val::Px(border_radius)))
}

fn sort_button(sort_by: KeyValue) -> impl Element {
    let button_holder = LazyEntity::new();
    let selected = SignalBuilder::from_resource::<SortBy>()
        .map_in(move |cur| cur.0 == sort_by)
        .dedupe();

    Row::<Node>::new()
        .with_node(|mut node| node.column_gap = Val::Px(35.))
        .align(Align::new().right())
        .item_signal::<Option<El<Text>>, _>(
            selected
                .clone()
                .map_bool_in(|| Some(sort_by_text_element()), || None),
        )
        .item(
            El::<Node>::new()
                .apply(border_radius_style(20.))
                .with_node(|mut node| {
                    node.width = Val::Px(200.);
                    node.height = Val::Px(80.);
                })
                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                .with_builder(|builder| builder.lazy_entity(button_holder.clone()))
                .background_color_signal(
                    SignalBuilder::from_lazy_entity(button_holder)
                        .has_component::<Hovered>()
                        .dedupe()
                        .combine(selected)
                        .dedupe()
                        .map_in(|(hovered, selected)| hovered || selected)
                        .map_bool_in(|| bevy::color::palettes::basic::GRAY.into(), || Color::BLACK)
                        .map_in(BackgroundColor)
                        .map_in(Some),
                )
                .align_content(Align::center())
                .on_click(
                    move |_: In<_>, mut sort_by_res: ResMut<SortBy>, pairs: Res<Pairs>, mut pairs_data: Query<&mut MutableVecData<RowData>>| {
                        if sort_by_res.0 != sort_by {
                            sort_by_res.0 = sort_by;
                        }
                        reorder_pairs(sort_by_res.0, &pairs.0, &mut pairs_data);
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

fn reorder_pairs(
    sort_by: KeyValue,
    pairs: &MutableVec<RowData>,
    pairs_data: &mut Query<&mut MutableVecData<RowData>>,
) {
    let mut guard = pairs.write(pairs_data);
    let mut desired = guard.to_vec();
    match sort_by {
        KeyValue::Key => desired.sort_by_key(|r| (r.key.is_empty(), r.key.clone())),
        KeyValue::Value => desired.sort_by_key(|r| (r.value.is_empty(), r.value.clone())),
    }

    let desired_ids = desired.iter().map(|r| r.id).collect::<Vec<_>>();
    for target_index in 0..desired_ids.len() {
        let target_id = desired_ids[target_index];

        let mut current_index = None;
        for i in 0..guard.len() {
            if guard[i].id == target_id {
                current_index = Some(i);
                break;
            }
        }

        if let Some(current_index) = current_index
            && current_index != target_index
        {
            guard.move_item(current_index, target_index);
        }
    }
}

fn key_values(pairs: MutableVec<RowData>, viewport_holder: LazyEntity) -> Column<Node> {
    Column::<Node>::new()
        .with_node(|mut node| {
            node.row_gap = Val::Px(10.);
            node.height = Val::Percent(90.);
        })
        .with_builder(|builder| builder.lazy_entity(viewport_holder.clone()))
        .mutable_viewport(haalka::prelude::Axis::Vertical)
        .on_scroll_with_system_on_hover(
            BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(20.)
                .into_system(),
        )
        .items_signal_vec(pairs.signal_vec().enumerate().map(clone!((pairs) move |In((index_signal, row)): In<(signal::Source<Option<usize>>, RowData)>| {
            let _ = index_signal;
                Row::<Node>::new()
                    .with_node(|mut node| {
                        node.column_gap = Val::Px(10.);
                        // without registering width up front, layout will take a frame or two to sync to size of
                        // children, making it look like the elements are expanding into place,
                        // try commenting out this line to see how it looks
                        node.width = Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.)
                    })
                    .item(text_input(pairs.clone(), row.id, Field::Key, row.key.clone(), row.autofocus_key))
                    .item(text_input(pairs.clone(), row.id, Field::Value, row.value.clone(), false))
                    .item(x_button().on_click(clone!((pairs) move |_: In<_>, mut input_focus: ResMut<InputFocus>, fields: Query<&RowField>, mut pairs_data: Query<&mut MutableVecData<RowData>>| {
                        // Clear focus if we're focused on an input for this row.
                        if let Some(focused) = input_focus.0
                            && let Ok(rf) = fields.get(focused)
                            && rf.row_id == row.id
                        {
                            input_focus.0 = None;
                        }
                        let mut guard = pairs.write(&mut pairs_data);
                        if let Some(i) = guard.iter().position(|r| r.id == row.id) {
                            guard.remove(i);
                        }
                    })))
            }
        )))
}

fn x_button() -> impl Element + PointerEventAware {
    let button_holder = LazyEntity::new();
    El::<Node>::new()
        .apply(border_radius_style(10.))
        .with_node(|mut node| {
            node.width = Val::Px(INPUT_HEIGHT);
            node.height = Val::Px(INPUT_HEIGHT);
        })
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .with_builder(|builder| builder.lazy_entity(button_holder.clone()))
        .background_color_signal(
            SignalBuilder::from_lazy_entity(button_holder)
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

fn ui_root(pairs: MutableVec<RowData>, viewport_holder: LazyEntity) -> impl Element {
    El::<Node>::new()
        .ui_root()
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
                            node.height = Val::Percent(90.);
                            node.width = Val::Px(INPUT_WIDTH * 2. + INPUT_HEIGHT + 10. * 2.);
                        })
                        .align_content(Align::center())
                        .item(key_values(pairs.clone(), viewport_holder.clone()).with_node(|mut node| node.height = Val::Percent(90.)))
                        .item({
                            let button_holder = LazyEntity::new();
                            El::<Node>::new()
                                .apply(border_radius_style(10.))
                                .with_node(|mut node| {
                                    node.width = Val::Px(INPUT_WIDTH);
                                    node.height = Val::Px(INPUT_HEIGHT);
                                })
                                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                                .with_builder(|builder| builder.lazy_entity(button_holder.clone()))
                                .background_color_signal(
                                    SignalBuilder::from_lazy_entity(button_holder)
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
                                .on_click(clone!((pairs, viewport_holder) move |_: In<_>, mut input_focus: ResMut<InputFocus>, mut next_id: ResMut<NextRowId>, mut pairs_data: Query<&mut MutableVecData<RowData>>, mut scroll_positions: Query<&mut ScrollPosition>| {
                                    input_focus.0 = None;
                                    let id = RowId(next_id.0);
                                    next_id.0 += 1;
                                    pairs.write(&mut pairs_data).push(RowData {
                                        id,
                                        key: String::new(),
                                        value: String::new(),
                                        autofocus_key: true,
                                    });
                                    if let Ok(mut sp) = scroll_positions.get_mut(viewport_holder.get()) {
                                        sp.y = f32::MAX;
                                    }
                                }))
                        }),
                ),
        )
}

fn autofocus(
    mut commands: Commands,
    mut input_focus: ResMut<InputFocus>,
    autofocused: Query<Entity, Added<AutoFocus>>,
) {
    for entity in autofocused.iter() {
        input_focus.0 = Some(entity);
        commands.entity(entity).remove::<AutoFocus>();
    }
}

fn tabber(
    keys: Res<ButtonInput<KeyCode>>,
    pairs: Res<Pairs>,
    pairs_data: Query<&MutableVecData<RowData>>,
    fields: Query<&RowField>,
    entities_with_fields: Query<(Entity, &RowField)>,
    mut input_focus: ResMut<InputFocus>,
) {
    // TODO: use .pressed instead of .just_pressed to allow for holding down tab, browser seems to
    // require minimum press time before starting to repeat, and repeating seems slower than refresh
    // rate
    let move_backward = keys.pressed(KeyCode::ShiftLeft) && keys.just_pressed(KeyCode::Tab);
    let move_forward = keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Enter);
    if !move_backward && !move_forward {
        return;
    }

    let current = input_focus.0.and_then(|entity| fields.get(entity).ok()).copied();
    let pairs_read = pairs.0.read(&pairs_data);

    let find_entity = |row_id: RowId, field: Field| -> Option<Entity> {
        entities_with_fields
            .iter()
            .find_map(|(entity, rf)| (rf.row_id == row_id && rf.field == field).then_some(entity))
    };

    if let Some(RowField { row_id, field }) = current {
        let row_index = pairs_read.iter().position(|r| r.id == row_id);
        if let Some(i) = row_index {
            let next = if move_backward {
                match field {
                    Field::Value => Some((row_id, Field::Key)),
                    Field::Key => {
                        if i > 0 {
                            Some((pairs_read[i - 1].id, Field::Value))
                        } else {
                            pairs_read.last().map(|r| (r.id, Field::Value))
                        }
                    }
                }
            } else {
                match field {
                    Field::Key => Some((row_id, Field::Value)),
                    Field::Value => {
                        if i + 1 < pairs_read.len() {
                            Some((pairs_read[i + 1].id, Field::Key))
                        } else {
                            pairs_read.first().map(|r| (r.id, Field::Key))
                        }
                    }
                }
            };
            if let Some((next_row, next_field)) = next
                && let Some(entity) = find_entity(next_row, next_field)
            {
                input_focus.0 = Some(entity);
            }
        }
    } else {
        // No focus: focus first/last depending on direction.
        let target = if move_backward {
            pairs_read.last().map(|r| (r.id, Field::Value))
        } else {
            pairs_read.first().map(|r| (r.id, Field::Key))
        };
        if let Some((row_id, field)) = target
            && let Some(entity) = find_entity(row_id, field)
        {
            input_focus.0 = Some(entity);
        }
    }
}

fn escaper(mut input_focus: ResMut<InputFocus>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        input_focus.0 = None;
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
