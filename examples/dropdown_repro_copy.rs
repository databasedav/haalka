//! Repro for dropdown one-frame delay using the same plugin stack as examples.

mod utils;
use utils::examples_plugin;

use bevy::{prelude::*, ui::Pressed};
use bevy_platform::collections::HashMap;
use haalka::{impl_haalka_methods, prelude::*};
use jonmo::prelude::*;
use std::fmt::Display;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FONT_SIZE: f32 = 25.0;
const BASE_PADDING: f32 = 10.0;
const DEFAULT_BUTTON_HEIGHT: f32 = 65.0;
const BASE_BORDER_WIDTH: f32 = 5.0;

#[derive(Component, Clone, Default)]
struct Selected;

#[derive(Default, Clone)]
struct Button {
    el: El<Node>,
}

impl ElementWrapper for Button {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl GlobalEventAware for Button {}
impl PointerEventAware for Button {}
impl BuilderPassThrough for Button {}

impl_haalka_methods! {
    Button {
        node: Node,
        background_color: BackgroundColor,
        border_color: BorderColor,
        border_radius: BorderRadius,
        z_index: ZIndex,
        visibility: Visibility,
    }
}

impl Button {
    fn new() -> Self {
        let lazy_entity = LazyEntity::new();

        let pressed = signal::from_entity(lazy_entity.clone())
            .map(|In(entity), presseds: Query<&Pressed>| presseds.contains(entity))
            .dedupe();

        let hovered = signal::from_entity(lazy_entity.clone())
            .has_component::<Hovered>()
            .dedupe();

        let selected = signal::from_entity(lazy_entity.clone())
            .has_component::<Selected>()
            .dedupe();

        let selected_hovered = signal::zip!(signal::any!(selected.clone(), pressed), hovered.clone()).dedupe();

        Self {
            el: {
                El::<Node>::new()
                    .lazy_entity(lazy_entity.clone())
                    .insert((Pickable::default(), Hoverable, Pressable))
                    .with_node(|mut node| {
                        node.height = Val::Px(DEFAULT_BUTTON_HEIGHT);
                        node.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                    })
                    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                    .align_content(Align::center())
                    .border_color_signal(
                        selected_hovered
                            .clone()
                            .map_in(|(selected, hovered)| {
                                if selected {
                                    bevy::color::palettes::basic::RED.into()
                                } else if hovered {
                                    Color::WHITE
                                } else {
                                    Color::BLACK
                                }
                            })
                            .map_in(BorderColor::all)
                            .map_in(Some),
                    )
                    .background_color_signal(
                        selected_hovered
                            .map_in(|(selected, hovered)| {
                                if selected {
                                    CLICKED_BUTTON
                                } else if hovered {
                                    HOVERED_BUTTON
                                } else {
                                    NORMAL_BUTTON
                                }
                            })
                            .map_in(BackgroundColor)
                            .map_in(Some),
                    )
            },
        }
    }

    fn body(mut self, body: impl Element) -> Self {
        self.el = self.el.child(body);
        self
    }

    fn selected_signal(mut self, selected: impl Signal<Item = bool> + Clone + Send + 'static) -> Self {
        self.el = self.el.component_signal(selected.map_true_in(|| Selected));
        self
    }
}

fn text_button<Marker>(
    text_signal: impl Signal<Item = String> + Clone + Send + 'static,
    on_click: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
) -> Button {
    Button::new()
        .body(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(text_signal.map_in(Text).map_in(Some)),
        )
        .on_click(on_click)
        .with_node(|mut node| node.width = Val::Px(200.0))
}

fn x_button() -> El<Node> {
    let lazy = LazyEntity::new();
    El::<Node>::new()
        .lazy_entity(lazy.clone())
        .background_color(BackgroundColor(Color::NONE))
        .insert((Pickable::default(), Hoverable))
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new("x"))
                .text_color_signal(
                    signal::from_entity(lazy)
                        .has_component::<Hovered>()
                        .map_bool_in(|| bevy::color::palettes::basic::RED.into(), || TEXT_COLOR)
                        .map_in(TextColor)
                        .map_in(Some),
                ),
        )
}

#[derive(Component, Clone, Default, Deref)]
struct DropdownSelectionIndex(Option<usize>);

#[derive(Component, Clone, Default)]
struct DropdownShowing;

#[derive(Component, Clone, Default)]
struct DropdownCloseRequested;

#[derive(Component, Clone, Default, Deref)]
struct DropdownHoveredIndex(Option<usize>);

#[derive(Component, Clone, Default)]
struct DropdownClearable;

#[derive(Component, Clone, Default, Deref)]
struct DropdownNumOptions(usize);

#[derive(Component, Clone, Copy)]
struct DropdownOptionsContainer {
    owner: Entity,
}

struct Dropdown<T: Display + Clone + PartialEq + Send + Sync + 'static> {
    el: El<Node>,
    lazy_entity: LazyEntity,
    options: Vec<T>,
    initial_selection: Option<T>,
    clearable: bool,
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> Dropdown<T> {
    fn new(options: Vec<T>) -> Self {
        Self {
            el: El::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            options,
            initial_selection: None,
            clearable: false,
        }
    }

    fn selection(mut self, selection: impl Into<Option<T>>) -> Self {
        self.initial_selection = selection.into();
        self
    }

    fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> ElementWrapper for Dropdown<T> {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            lazy_entity,
            options,
            initial_selection,
            clearable,
        } = self;

        let show = signal::from_entity(lazy_entity.clone())
            .has_component::<DropdownShowing>()
            .dedupe();
        let opts = options.clone();
        let opts_for_text = opts.clone();
        let opts_for_list = opts.clone();
        let num_options = options.len();
        let initial_idx = initial_selection.and_then(|s| options.iter().position(|o| *o == s));

        el.lazy_entity(lazy_entity.clone())
            .with_builder(move |builder| {
                let mut b = builder
                    .insert(DropdownSelectionIndex(initial_idx))
                    .insert(DropdownNumOptions(num_options))
                    .insert(DropdownHoveredIndex(None));
                if clearable {
                    b = b.insert(DropdownClearable);
                }
                b
            })
            .child(
                Button::new()
                    .with_node(|mut node| node.width = Val::Px(300.0))
                    .body(
                        Stack::<Node>::new()
                            .with_node(|mut node| {
                                node.width = Val::Percent(100.0);
                                node.padding = UiRect::horizontal(Val::Px(BASE_PADDING));
                            })
                            .layer({
                                let opts_for_text = opts_for_text.clone();
                                El::<Text>::new()
                                    .align(Align::new().left())
                                    .text_font(TextFont::from_font_size(FONT_SIZE))
                                    .text_signal(
                                        signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity.clone())
                                            .map_in(deref_copied)
                                            .map_some_in(move |i| opts_for_text[i].to_string())
                                            .map_in(Option::unwrap_or_default)
                                            .map_in(Text)
                                            .map_in(Some),
                                    )
                            })
                            .layer({
                                let x_button = || {
                                    signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity.clone())
                                        .map_in(deref_copied)
                                        .map_in_ref(Option::is_some)
                                        .map_true_in(clone!((lazy_entity) move || {
                                            x_button()
                                                .on_click(clone!((lazy_entity) move |_: In<_>, mut selections: Query<(&mut DropdownSelectionIndex, &mut DropdownHoveredIndex)>| {
                                                    if let Ok((mut sel, mut hovered)) = selections.get_mut(*lazy_entity) {
                                                        sel.0 = None;
                                                        hovered.0 = None;
                                                    }
                                                }))
                                                .observe(|mut click: On<Pointer<Click>>| click.propagate(false))
                                        }))
                                };
                                let mut el = Row::<Node>::new()
                                    .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING))
                                    .align(Align::new().right());
                                if clearable {
                                    el = el.item_signal(x_button());
                                }
                                el.item(
                                    El::<Text>::new()
                                        .text_font(TextFont::from_font_size(FONT_SIZE))
                                        .text(Text::new("v")),
                                )
                            }),
                    )
                    .on_click(clone!((lazy_entity) move |_: In<_>, showings: Query<&DropdownShowing>, frames: Res<FrameCount>, mut commands: Commands| {
                        let frame = frames.0;
                        info!("frame={} dropdown button click: owner={:?}", frame, *lazy_entity);
                        if showings.contains(*lazy_entity) {
                            commands.entity(*lazy_entity).remove::<DropdownShowing>();
                        } else {
                            commands.entity(*lazy_entity).insert(DropdownShowing);
                        }
                    })),
            )
            .child_signal(
                show.map_true(clone!((lazy_entity, opts_for_list) move |In(_), world: &mut World| {
                    Column::<Node>::new()
                        .insert((Pickable::default(), DropdownOptionsContainer { owner: *lazy_entity }))
                        .global_z_index(GlobalZIndex(i32::MAX))
                        .with_builder(|builder| {
                            builder
                                .on_spawn(clone!((lazy_entity) move |world, entity| {
                                    let frame = world.resource::<FrameCount>().0;
                                    info!(
                                        "frame={} dropdown options on_spawn: owner={:?} options_entity={:?}",
                                        frame, *lazy_entity, entity
                                    );
                                }))
                                .on_despawn(|world, entity| {
                                    let frame = world.resource::<FrameCount>().0;
                                    info!(
                                        "frame={} dropdown options on_despawn: options_entity={:?}",
                                        frame, entity
                                    );
                                })
                        })
                        .with_node(|mut node| {
                            node.width = Val::Percent(100.0);
                            node.top = Val::Percent(100.0);
                            node.position_type = PositionType::Absolute;
                        })
                        .on_click_outside(clone!((lazy_entity) move |In((_entity, click)): In<(Entity, GlobalEventData<Pointer<Click>>)>,
                            frames: Res<FrameCount>, mut commands: Commands| {
                            let frame = frames.0;
                            info!(
                                "frame={} dropdown click outside: owner={:?} original_target={:?} location={:?}",
                                frame, *lazy_entity, click.original_event_target, click.event.pointer_location
                            );
                            commands.entity(*lazy_entity).insert(DropdownCloseRequested);
                        }))
                        .items_signal_vec(
                            MutableVec::builder()
                                .values(opts_for_list.iter().cloned().enumerate().collect::<Vec<_>>())
                                .spawn(world)
                                .signal_vec()
                                .map_in(clone!((lazy_entity) move |(i, opt): (usize, T)| {
                                    text_button(
                                        signal::once(opt.to_string()),
                                        clone!((lazy_entity) move |_: In<_>, mut dropdowns: Query<&mut DropdownSelectionIndex>, mut commands: Commands| {
                                            dropdowns.get_mut(*lazy_entity).unwrap().0 = Some(i);
                                            commands.entity(*lazy_entity).remove::<DropdownShowing>();
                                        }),
                                    )
                                    .with_node(|mut node| node.width = Val::Percent(100.0))
                                    .selected_signal(
                                        signal::from_component_changed::<DropdownHoveredIndex>(lazy_entity.clone())
                                            .map_in(deref_copied)
                                            .eq(Some(i)),
                                    )
                                })),
                        )
                })),
            )
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> BuilderPassThrough for Dropdown<T> {}

#[derive(Resource)]
struct DropdownEntities {
    first: LazyEntity,
    second: LazyEntity,
}

#[derive(Resource, Default)]
struct FrameCount(u32);

#[derive(Resource, Default)]
struct ExitAfter(u32);

fn tick_frame_count(mut frames: ResMut<FrameCount>) {
    frames.0 += 1;
}

fn log_showing_changes(
    frames: Res<FrameCount>,
    added: Query<Entity, Added<DropdownShowing>>,
    mut removed: RemovedComponents<DropdownShowing>,
) {
    for entity in added.iter() {
        info!("frame={} dropdown showing added: owner={:?}", frames.0, entity);
    }
    for entity in removed.read() {
        info!("frame={} dropdown showing removed: owner={:?}", frames.0, entity);
    }
}

fn log_options_added(
    frames: Res<FrameCount>,
    added: Query<(Entity, &DropdownOptionsContainer), Added<DropdownOptionsContainer>>,
) {
    for (entity, container) in added.iter() {
        info!(
            "frame={} dropdown options spawned: owner={:?} options_entity={:?}",
            frames.0, container.owner, entity
        );
    }
}

fn apply_dropdown_close_requests(
    mut commands: Commands,
    requests: Query<Entity, With<DropdownCloseRequested>>,
) {
    for entity in requests.iter() {
        commands.entity(entity).remove::<DropdownShowing>();
        commands.entity(entity).remove::<DropdownCloseRequested>();
    }
}

fn toggle_dropdowns(
    mut commands: Commands,
    entities: Res<DropdownEntities>,
    frames: Res<FrameCount>,
) {
    if frames.0 == 1 {
        commands.entity(*entities.second).insert(DropdownShowing);
    } else if frames.0 == 2 {
        commands.entity(*entities.second).remove::<DropdownShowing>();
        commands.entity(*entities.first).insert(DropdownShowing);
    }
}

fn dropdown_debug_log(
    showings: Query<(), With<DropdownShowing>>,
    options: Query<(Entity, &DropdownOptionsContainer, Option<&Children>)>,
    mut last_counts: Local<HashMap<Entity, usize>>,
    frame: Res<FrameCount>,
) {
    for (entity, container, children) in options.iter() {
        if !showings.contains(container.owner) {
            continue;
        }
        let count = children.map(|c| c.len()).unwrap_or(0);
        let prev = last_counts.get(&entity).copied().unwrap_or(usize::MAX);
        if prev != count {
            info!(
                "frame={} dropdown options children changed: owner={:?} options_entity={:?} count={}",
                frame.0, container.owner, entity, count
            );
            last_counts.insert(entity, count);
        }
        if count == 0 {
            warn!(
                "frame={} dropdown options empty: owner={:?} options_entity={:?}",
                frame.0, container.owner, entity
            );
        }
    }
}

fn dropdown_computed_log(
    showings: Query<(), With<DropdownShowing>>,
    options: Query<(Entity, &DropdownOptionsContainer, Option<&ComputedNode>)>,
    mut last_sizes: Local<HashMap<Entity, Vec2>>,
    frame: Res<FrameCount>,
) {
    for (entity, container, computed) in options.iter() {
        if !showings.contains(container.owner) {
            continue;
        }
        let size = computed.map(|c| c.size).unwrap_or(Vec2::ZERO);
        let prev = last_sizes.get(&entity).copied().unwrap_or(Vec2::new(-1.0, -1.0));
        if prev != size {
            info!(
                "frame={} dropdown options computed size changed: owner={:?} options_entity={:?} size={:?}",
                frame.0, container.owner, entity, size
            );
            last_sizes.insert(entity, size);
        }
    }
}

fn exit_after_frames(
    frame: Res<FrameCount>,
    exit_after: Res<ExitAfter>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if frame.0 >= exit_after.0 {
        app_exit.write(AppExit::Success);
    }
}

fn ui_root(entities: &DropdownEntities) -> El<Node> {
    let dropdown_one = Dropdown::new(vec![
        "Low".to_string(),
        "Medium".to_string(),
        "High".to_string(),
    ])
    .selection(Some("Medium".to_string()))
    .clearable()
    .lazy_entity(entities.first.clone())
    .insert(ZIndex(i32::MAX));

    let dropdown_two = Dropdown::new(vec![
        "One".to_string(),
        "Two".to_string(),
        "Three".to_string(),
        "Four".to_string(),
    ])
    .selection(Some("One".to_string()))
    .lazy_entity(entities.second.clone())
    .insert(ZIndex(i32::MAX-1));

    let dropdown_three = Dropdown::new(vec![
        "One".to_string(),
        "Two".to_string(),
        "Three".to_string(),
        "Four".to_string(),
    ])
    .selection(Some("One".to_string()));
    // .lazy_entity(entities.second.clone());

    El::<Node>::new()
        .ui_root()
        .with_node(|mut node| {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        })
        .align_content(Align::center())
        .child(Column::<Node>::new().items([dropdown_one, dropdown_two, dropdown_three]))
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(Startup, (camera,))
        // .add_systems(
        //     Update,
        //     (
        //         tick_frame_count,
        //         toggle_dropdowns,
        //         log_showing_changes,
        //         log_options_added,
        //         exit_after_frames,
        //     )
        //         .chain(),
        // )
        .add_systems(
            PostUpdate,
            (
                dropdown_debug_log.after(jonmo::SignalProcessing),
                apply_dropdown_close_requests.after(jonmo::SignalProcessing),
                dropdown_computed_log.after(bevy_ui::UiSystems::Layout),
            ),
        )
        .init_resource::<FrameCount>()
        .insert_resource(ExitAfter(120))
        .insert_resource(DropdownEntities {
            first: LazyEntity::new(),
            second: LazyEntity::new(),
        })
        .add_systems(Startup, |world: &mut World| {
            let entities = world.resource::<DropdownEntities>();
            ui_root(entities).spawn(world);
        })
        .run();
}
