//! - Main menu with sub menus for audio and graphics.
//! - Simple buttons for option selection.
//! - Slider for volume.
//! - Dropdown for graphics quality (low/medium/high).
//! - Navigation possible with mouse, keyboard and controller.
//!   - Mouse: Separate styles for hover and press.
//!   - Keyboard/Controller: Separate styles for currently focused element.

mod utils;
use bevy_ecs::entity;
use jonmo::graph::{downcast_any_clone, poll_signal};
use utils::*;

use bevy::{prelude::*, ui::Pressed};
use haalka::{impl_haalka_methods, prelude::*};
use std::{fmt::Display, i32};
use strum::{Display as StrumDisplay, EnumIter, IntoEnumIterator};

// Note: For actual serialization, add serde as a dependency and derive Serialize/Deserialize

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
        .add_systems(Update, (keyboard_menu_input_events, gamepad_menu_input_events))
        .init_resource::<SubMenuSelected>()
        .init_resource::<ShowSubMenu>()
        .insert_resource(AudioSettings {
            dropdown: None,
            radio_group: None,
            checkbox: false,
            iterable_option: "option 1".to_string(),
            master_volume: 100.,
            effect_volume: 50.,
            music_volume: 50.,
            voice_volume: 50.,
        })
        .insert_resource(GraphicsSettings {
            texture_quality: Quality::Medium,
            shadow_quality: Quality::Medium,
            bloom_quality: Quality::Medium,
        })
        .insert_resource(MenuInputRateLimiter(Timer::from_seconds(
            MENU_INPUT_RATE_LIMIT,
            TimerMode::Repeating,
        )))
        .insert_resource(SliderRateLimiter(Timer::from_seconds(
            SLIDER_RATE_LIMIT,
            TimerMode::Repeating,
        )))
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FONT_SIZE: f32 = 25.;
const MAIN_MENU_SIDES: f32 = 300.;
const SUB_MENU_HEIGHT: f32 = 700.;
const SUB_MENU_WIDTH: f32 = 1200.;
const BASE_PADDING: f32 = 10.;
const DEFAULT_BUTTON_HEIGHT: f32 = 65.;
const BASE_BORDER_WIDTH: f32 = 5.;
const MENU_ITEM_HEIGHT: f32 = DEFAULT_BUTTON_HEIGHT + BASE_PADDING;
const LIL_BABY_BUTTON_SIZE: f32 = 30.;

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter)]
enum SubMenu {
    Audio,
    Graphics,
}

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref)]
struct ShowSubMenu(Option<SubMenu>);

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref)]
struct SubMenuSelected(Option<SubMenu>);

/// Resource for audio settings - easily serializable
#[derive(Resource, Clone, PartialEq, Default)]
struct AudioSettings {
    dropdown: Option<String>,
    radio_group: Option<usize>,
    checkbox: bool,
    iterable_option: String,
    master_volume: f32,
    effect_volume: f32,
    music_volume: f32,
    voice_volume: f32,
}

/// Resource for graphics settings - easily serializable
#[derive(Resource, Clone, PartialEq, Default)]
struct GraphicsSettings {
    texture_quality: Quality,
    shadow_quality: Quality,
    bloom_quality: Quality,
}

/// Component to mark a button as selected (for external control)
#[derive(Component, Clone, Default)]
struct Selected;

// core widget, pretty much every other widget uses the `Button`
#[derive(Default, Clone)]
struct Button {
    el: El<Node>,
}

// implementing `ElementWrapper` allows the struct to be passed directly to .child methods
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

    fn hovered_signal(mut self, hovered: impl Signal<Item = bool> + Clone + Send + 'static) -> Self {
        self.el = self.el.component_signal(hovered.map_true_in(|| Hovered));
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
                .text(Text::new("test"))
                // .text_signal(text_signal.map_in(Text).map_in(Some)),
        )
        .on_click(on_click)
        .with_node(|mut node| node.width = Val::Px(200.))
}

fn sub_menu_button(sub_menu: SubMenu) -> Button {
    Button::new()
        .body(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(sub_menu.to_string())),
        )
        .on_click(move |_: In<_>, mut commands: Commands| {
            commands.insert_resource(ShowSubMenu(Some(sub_menu)));
        })
        .with_node(|mut node| node.width = Val::Px(200.))
}

fn menu_base(width: f32, height: f32, title: &str) -> Column<Node> {
    Column::<Node>::new()
        .with_node(move |mut node| {
            node.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
            node.width = Val::Px(width);
            node.height = Val::Px(height);
        })
        .border_color(BorderColor::all(Color::BLACK))
        .background_color(BackgroundColor(NORMAL_BUTTON))
        .item(
            El::<Node>::new()
                .with_node(|mut node| {
                    node.height = Val::Px(MENU_ITEM_HEIGHT);
                    node.padding = UiRect::all(Val::Px(BASE_PADDING * 2.));
                })
                .child(
                    El::<Text>::new()
                        .align(Align::new().top().left())
                        .text_font(TextFont::from_font_size(FONT_SIZE))
                        .text(Text::new(title)),
                ),
        )
}

fn lil_baby_button() -> Button {
    Button::new().with_node(|mut node| {
        node.width = Val::Px(LIL_BABY_BUTTON_SIZE);
        node.height = Val::Px(LIL_BABY_BUTTON_SIZE);
    })
}

/// Component marker for the currently focused widget (receives MenuInputEvents)
#[derive(Component, Clone, Default)]
struct Focused;

/// Component on ancestor containers that determines which child index is focused
#[derive(Component, Clone, Default, Deref, DerefMut)]
struct FocusedIndex(Option<usize>);

/// Walks up the hierarchy to find an ancestor with `FocusedIndex`, caches it, then returns
/// a signal that watches that ancestor's `FocusedIndex` component.
fn signal_from_ancestor_focused_index(lazy_entity: LazyEntity) -> impl Signal<Item = Option<usize>> + Clone {
    signal::from_entity(lazy_entity)
        .map(
            |In(entity): In<Entity>, child_ofs: Query<&ChildOf>, focused_indices: Query<(), With<FocusedIndex>>| {
                child_ofs
                    .iter_ancestors(entity)
                    .find(|&ancestor| focused_indices.contains(ancestor))
                    .unwrap()
            },
        )
        .first()
        .switch(|In(ancestor): In<Entity>| {
            signal::from_component_changed::<FocusedIndex>(ancestor).map_in(deref_copied)
        })
}

/// Component to store checkbox checked state
#[derive(Component, Clone, Default)]
struct Checked;

struct Checkbox {
    el: El<Node>,
    lazy_entity: LazyEntity,
    initially_checked: bool,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl Checkbox {
    fn new() -> Self {
        Self {
            el: El::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            initially_checked: false,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn checked(mut self, checked: bool) -> Self {
        self.initially_checked = checked;
        self
    }

    /// Sync the checkbox state from an external signal
    fn checked_signal(mut self, signal: impl Signal<Item = bool> + Clone + Send + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal
            .dedupe()
            .map(
                clone!((lazy_entity) move |In(checked): In<bool>, checkeds: Query<&Checked>, mut commands: Commands| {
                    let entity = *lazy_entity;
                    let is_checked = checkeds.contains(entity);
                    if checked && !is_checked {
                        commands.entity(entity).insert(Checked);
                    } else if !checked && is_checked {
                        commands.entity(entity).remove::<Checked>();
                    }
                }),
            )
            .task();
        self.external_sync_task = Some(task);
        self
    }

    /// Called when the checkbox state changes
    fn on_change<M>(mut self, handler: impl IntoSystem<In<bool>, (), M> + Send + Sync + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal::from_component_changed::<Checked>(lazy_entity.clone())
            .map_in(|_| ())
            .switch(clone!((lazy_entity) move |_: In<()>| {
                signal::from_entity(lazy_entity.clone())
                    .has_component::<Checked>()
                    .first()
            }))
            .map(handler)
            .task();
        self.on_change_task = Some(task);
        self
    }
}

impl ElementWrapper for Checkbox {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el: _,
            lazy_entity,
            initially_checked,
            external_sync_task,
            on_change_task,
        } = self;

        lil_baby_button()
            .with_builder(clone!((lazy_entity) move |builder| {
                let mut b = builder.lazy_entity(lazy_entity.clone());
                if initially_checked {
                    b = b.insert(Checked);
                }
                if let Some(task) = external_sync_task {
                    b = b.hold_tasks([task]);
                }
                if let Some(task) = on_change_task {
                    b = b.hold_tasks([task]);
                }
                b
            }))
            .observe(clone!((lazy_entity) move |event: On<MenuInputEvent>,
                  checkeds: Query<&Checked>,
                  mut commands: Commands| {
                let entity = *lazy_entity;
                let is_checked = checkeds.contains(entity);
                match event.input {
                    MenuInput::Select => {
                        if is_checked {
                            commands.entity(entity).remove::<Checked>();
                        } else {
                            commands.entity(entity).insert(Checked);
                        }
                    }
                    MenuInput::Delete => {
                        commands.entity(entity).remove::<Checked>();
                    }
                    _ => (),
                }
            }))
            .on_click(
                clone!((lazy_entity) move |_: In<_>, checkeds: Query<&Checked>, mut commands: Commands| {
                    let entity = *lazy_entity;
                    if checkeds.contains(entity) {
                        commands.entity(entity).remove::<Checked>();
                    } else {
                        commands.entity(entity).insert(Checked);
                    }
                }),
            )
            .selected_signal(
                signal::from_entity(lazy_entity.clone())
                    .has_component::<Checked>()
                    .dedupe(),
            )
            .el
    }
}

impl BuilderPassThrough for Checkbox {}

#[derive(Clone, Copy, EnumIter, PartialEq, StrumDisplay, Default)]
enum Quality {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl Quality {
    fn from_string(s: &str) -> Option<Self> {
        match s {
            "Low" => Some(Quality::Low),
            "Medium" => Some(Quality::Medium),
            "High" => Some(Quality::High),
            "Ultra" => Some(Quality::Ultra),
            _ => None,
        }
    }
}

/// Component to store radio group selection
#[derive(Component, Clone, Default, Deref)]
struct RadioSelection(Option<usize>);

/// Component to store the radio group's options
#[derive(Component, Clone)]
struct RadioOptions(Vec<String>);

struct RadioGroup {
    el: Row<Node>,
    lazy_entity: LazyEntity,
    options: Vec<String>,
    initial_selection: Option<usize>,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl RadioGroup {
    fn new(options: Vec<String>) -> Self {
        Self {
            el: Row::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            options,
            initial_selection: None,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn selection(mut self, selection: impl Into<Option<usize>>) -> Self {
        self.initial_selection = selection.into();
        self
    }

    /// Sync the selection from an external signal
    fn selection_signal(mut self, signal: impl Signal<Item = Option<usize>> + Clone + Send + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal
            .dedupe()
            .map(clone!((lazy_entity) move |In(selection): In<Option<usize>>, mut selections: Query<&mut RadioSelection>| {
                if let Ok(mut sel) = selections.get_mut(*lazy_entity) {
                    if sel.0 != selection {
                        sel.0 = selection;
                    }
                }
            }))
            .task();
        self.external_sync_task = Some(task);
        self
    }

    /// Called when the selection changes
    fn on_change<M>(mut self, handler: impl IntoSystem<In<Option<usize>>, (), M> + Send + Sync + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal::from_component_changed::<RadioSelection>(lazy_entity)
            .map_in(deref_copied)
            .map(handler)
            .task();
        self.on_change_task = Some(task);
        self
    }
}

impl ElementWrapper for RadioGroup {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            lazy_entity,
            options,
            initial_selection,
            external_sync_task,
            on_change_task,
        } = self;

        let opts = options.clone();
        el.with_builder(clone!((lazy_entity, options) move |builder| {
            let mut b = builder
                .lazy_entity(lazy_entity.clone())
                .insert(RadioSelection(initial_selection))
                .insert(RadioOptions(options));
            if let Some(task) = external_sync_task {
                b = b.hold_tasks([task]);
            }
            if let Some(task) = on_change_task {
                b = b.hold_tasks([task]);
            }
            b
        }))
        .observe(clone!((lazy_entity) move |event: On<MenuInputEvent>,
              mut selections: Query<(&mut RadioSelection, &RadioOptions)>| {
            let Ok((mut selection, options)) = selections.get_mut(*lazy_entity) else {
                return;
            };
            match event.input {
                MenuInput::Left | MenuInput::Right => {
                    let selected_option = selection.0;
                    let (mut i, step) = if matches!(event.input, MenuInput::Left) {
                        (selected_option.unwrap_or(options.0.len() - 1) as isize, -1)
                    } else {
                        (selected_option.unwrap_or(0) as isize, 1)
                    };
                    if selected_option.is_some() {
                        i = (i + step + options.0.len() as isize) % options.0.len() as isize;
                    }
                    selection.0 = Some(i as usize);
                }
                MenuInput::Delete => {
                    selection.0 = None;
                }
                _ => (),
            }
        }))
        .items(
            opts.into_iter()
                .enumerate()
                .map(clone!((lazy_entity) move |(i, option)| {
                    text_button(
                        signal::always(option.clone()),
                        clone!((lazy_entity) move |_: In<_>, mut selections: Query<&mut RadioSelection>| {
                            if let Ok(mut selection) = selections.get_mut(*lazy_entity) {
                                if selection.0 == Some(i) {
                                    selection.0 = None;
                                } else {
                                    selection.0 = Some(i);
                                }
                            }
                        }),
                    )
                    .selected_signal(
                        signal::from_component_changed::<RadioSelection>(lazy_entity.clone())
                            .map_in(deref_copied)
                            .eq(Some(i)),
                    )
                })),
        )
    }
}

impl BuilderPassThrough for RadioGroup {}

enum LeftRight {
    Left,
    Right,
}

fn arrow_text(direction: LeftRight) -> El<Text> {
    El::<Text>::new()
        .text_font(TextFont::from_font_size(FONT_SIZE))
        .text(Text::new(match direction {
            LeftRight::Left => "<",
            LeftRight::Right => ">",
        }))
}

/// Component to store iterable options current selection
#[derive(Component, Clone, Deref)]
struct IterableSelection(String);

/// Component to store the iterable options
#[derive(Component, Clone)]
struct IterableOptionsList(Vec<String>);

/// Component markers for left/right button press state
#[derive(Component, Clone, Default)]
struct LeftPressed;

#[derive(Component, Clone, Default)]
struct RightPressed;

/// Timer components for delayed removal of press states
#[derive(Component)]
struct LeftPressedTimer(#[allow(dead_code)] Timer);

#[derive(Component)]
struct RightPressedTimer(#[allow(dead_code)] Timer);

const FLASH_MS: f32 = 50.; // TODO: address background/border color desyncing

struct IterableOptions {
    el: Row<Node>,
    lazy_entity: LazyEntity,
    options: Vec<String>,
    initial_selection: String,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl IterableOptions {
    fn new(options: Vec<String>) -> Self {
        let initial_selection = options.first().cloned().unwrap_or_default();
        Self {
            el: Row::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            options,
            initial_selection,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn selection(mut self, selection: impl Into<String>) -> Self {
        self.initial_selection = selection.into();
        self
    }

    /// Sync the selection from an external signal
    fn selection_signal(mut self, signal: impl Signal<Item = String> + Clone + Send + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal
            .dedupe()
            .map(
                clone!((lazy_entity) move |In(selection): In<String>, mut iterables: Query<&mut IterableSelection>| {
                    if let Ok(mut sel) = iterables.get_mut(*lazy_entity) {
                        if sel.0 != selection {
                            sel.0 = selection;
                        }
                    }
                }),
            )
            .task();
        self.external_sync_task = Some(task);
        self
    }

    /// Called when the selection changes
    fn on_change<M>(mut self, handler: impl IntoSystem<In<String>, (), M> + Send + Sync + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal::from_component_changed::<IterableSelection>(lazy_entity)
            .map_in(deref_cloned)
            .map(handler)
            .task();
        self.on_change_task = Some(task);
        self
    }
}

impl ElementWrapper for IterableOptions {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            lazy_entity,
            options,
            initial_selection,
            external_sync_task,
            on_change_task,
        } = self;

        let opts = options.clone();
        el
            .with_builder(clone!((lazy_entity, options, initial_selection) move |builder| {
                let mut b = builder
                    .lazy_entity(lazy_entity.clone())
                    .insert(IterableSelection(initial_selection))
                    .insert(IterableOptionsList(options));
                if let Some(task) = external_sync_task {
                    b = b.hold_tasks([task]);
                }
                if let Some(task) = on_change_task {
                    b = b.hold_tasks([task]);
                }
                b
            }))
            .observe(
                clone!((lazy_entity) move |event: On<MenuInputEvent>,
                      mut iterables: Query<(&mut IterableSelection, &IterableOptionsList)>,
                      mut commands: Commands| {
                    let Ok((mut selection, options)) = iterables.get_mut(*lazy_entity) else {
                        return;
                    };
                    match event.input {
                        MenuInput::Left | MenuInput::Right => {
                            let i_option = options.0.iter().position(|opt| opt == &selection.0).map(|i| i as isize);
                            if let Some(mut i) = i_option {
                                let step = if matches!(event.input, MenuInput::Left) {
                                    commands.entity(*lazy_entity).insert((
                                        LeftPressed,
                                        LeftPressedTimer(Timer::from_seconds(FLASH_MS / 1000., TimerMode::Once)),
                                    ));
                                    -1
                                } else {
                                    commands.entity(*lazy_entity).insert((
                                        RightPressed,
                                        RightPressedTimer(Timer::from_seconds(FLASH_MS / 1000., TimerMode::Once)),
                                    ));
                                    1
                                };
                                i = (i + step + options.0.len() as isize) % options.0.len() as isize;
                                selection.0 = options.0[i as usize].clone();
                            }
                        }
                        _ => (),
                    }
                }),
            )
            .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING * 2.))
            .item({
                lil_baby_button()
                    .selected_signal(
                        signal::from_entity(lazy_entity.clone())
                            .has_component::<LeftPressed>()
                            .dedupe(),
                    )
                    .on_click(clone!((lazy_entity, opts) move |_: In<_>, mut iterables: Query<(&mut IterableSelection, &IterableOptionsList)>| {
                        let Ok((mut selection, options)) = iterables.get_mut(*lazy_entity) else {
                            return;
                        };
                        if let Some(i) = options.0.iter().position(|opt| opt == &selection.0) {
                            let new_i = if i == 0 { options.0.len() - 1 } else { i - 1 };
                            selection.0 = options.0[new_i].clone();
                        }
                    }))
                    .body(arrow_text(LeftRight::Left))
            })
            .item(
                El::<Text>::new()
                    .text_font(TextFont::from_font_size(FONT_SIZE))
                    .text_signal(
                        signal::from_component_changed::<IterableSelection>(lazy_entity.clone())
                            .map_in(deref_cloned)
                            .map_in(Text)
                            .map_in(Some),
                    ),
            )
            .item({
                lil_baby_button()
                    .selected_signal(
                        signal::from_entity(lazy_entity.clone())
                            .has_component::<RightPressed>()
                            .dedupe(),
                    )
                    .on_click(clone!((lazy_entity, opts) move |_: In<_>, mut iterables: Query<(&mut IterableSelection, &IterableOptionsList)>| {
                        let Ok((mut selection, options)) = iterables.get_mut(*lazy_entity) else {
                            return;
                        };
                        if let Some(i) = options.0.iter().position(|opt| opt == &selection.0) {
                            let new_i = (i + 1) % options.0.len();
                            selection.0 = options.0[new_i].clone();
                        }
                    }))
                    .body(arrow_text(LeftRight::Right))
            })
    }
}

impl BuilderPassThrough for IterableOptions {}

/// Component to store slider value
#[derive(Component, Clone, Deref)]
struct SliderValue(f32);

/// Component marker for slider being dragged
#[derive(Component, Clone, Default)]
struct SliderDragging;

const SLIDER_WIDTH: f32 = 400.;
const SLIDER_PADDING: f32 = 5.;
const SLIDER_MAX: f32 = SLIDER_WIDTH - SLIDER_PADDING - LIL_BABY_BUTTON_SIZE - BASE_BORDER_WIDTH;

struct Slider {
    el: Row<Node>,
    lazy_entity: LazyEntity,
    initial_value: f32,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl Slider {
    fn new() -> Self {
        Self {
            el: Row::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            initial_value: 0.,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn value(mut self, value: f32) -> Self {
        self.initial_value = value;
        self
    }

    /// Sync the slider value from an external signal
    fn value_signal(mut self, signal: impl Signal<Item = f32> + Clone + Send + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal
            .dedupe()
            .map(
                clone!((lazy_entity) move |In(value): In<f32>, mut sliders: Query<&mut SliderValue>| {
                    let Ok(mut slider_value) = sliders.get_mut(*lazy_entity) else {
                        return;
                    };
                    if (slider_value.0 - value).abs() > 0.01 {
                        slider_value.0 = value;
                    }
                }),
            )
            .task();
        self.external_sync_task = Some(task);
        self
    }

    /// Called when the slider value changes
    fn on_change<M>(mut self, handler: impl IntoSystem<In<f32>, (), M> + Send + Sync + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let task = signal::from_component_changed::<SliderValue>(lazy_entity)
            .map_in(deref_copied)
            .map(handler)
            .task();
        self.on_change_task = Some(task);
        self
    }
}

impl ElementWrapper for Slider {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            lazy_entity,
            initial_value,
            external_sync_task,
            on_change_task,
        } = self;

        let max = SLIDER_MAX;

        el
            .insert(SliderTag)
            .with_builder(clone!((lazy_entity) move |builder| {
                let mut b = builder
                    .lazy_entity(lazy_entity.clone())
                    .insert(SliderValue(initial_value));
                if let Some(task) = external_sync_task {
                    b = b.hold_tasks([task]);
                }
                if let Some(task) = on_change_task {
                    b = b.hold_tasks([task]);
                }
                b
            }))
            .observe(
                clone!((lazy_entity) move |event: On<MenuInputEvent>,
                      mut sliders: Query<&mut SliderValue>| {
                    let Ok(mut value) = sliders.get_mut(*lazy_entity) else {
                        return;
                    };
                    match event.input {
                        MenuInput::Left | MenuInput::Right => {
                            let dir = if matches!(event.input, MenuInput::Left) { -1. } else { 1. };
                            value.0 = (value.0 + dir * 0.1).clamp(0., 100.);
                        }
                        _ => (),
                    }
                }),
            )
            .with_node(|mut node| node.column_gap = Val::Px(10.))
            .item(
                El::<Text>::new()
                    .text_font(TextFont::from_font_size(FONT_SIZE))
                    .text_signal(
                        signal::from_component_changed::<SliderValue>(lazy_entity.clone())
                            .map_in(deref_copied)
                            .map_in(|value| format!("{value:.1}"))
                            .map_in(Text)
                            .map_in(Some),
                    ),
            )
            .item(
                Stack::<Node>::new()
                    .with_node(move |mut node| {
                        node.width = Val::Px(SLIDER_WIDTH);
                        node.height = Val::Px(5.);
                        node.padding = UiRect::horizontal(Val::Px(SLIDER_PADDING));
                    })
                    .background_color(BackgroundColor(Color::BLACK))
                    .layer({
                        let knob_entity = LazyEntity::new();
                        lil_baby_button()
                            .selected_signal(
                                signal::from_entity(knob_entity.clone())
                                    .has_component::<SliderDragging>()
                                    .dedupe(),
                            )
                            .el // we need lower level access now
                            .lazy_entity(knob_entity.clone())
                            .insert(Pickable::default())
                            .on_signal_with_node(
                                signal::from_component_changed::<SliderValue>(lazy_entity.clone())
                                    .map_in(deref_copied),
                                move |mut node, value| node.left = Val::Px(value / 100. * max),
                            )
                            .align(Align::new().center_y())
                            .on_dragged(clone!((knob_entity, lazy_entity) move |In((_, drag)): In<(Entity, DragData)>, mut sliders: Query<&mut SliderValue>, mut commands: Commands| {
                                if drag.dragged {
                                    commands.entity(*knob_entity).insert(SliderDragging);
                                    let Ok(mut value) = sliders.get_mut(*lazy_entity) else {
                                        return;
                                    };
                                    value.0 = (value.0 + drag.delta.x / max * 100.).clamp(0., 100.);
                                } else {
                                    commands.entity(*knob_entity).remove::<SliderDragging>();
                                }
                            }))
                    }),
            )
    }
}

impl BuilderPassThrough for Slider {}

fn options(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("option {i}")).collect()
}

/// Component marker for menu items that are hovered
#[derive(Component, Clone, Default)]
struct MenuItemFocused;

fn menu_item(label: &str, body: impl Element) -> Stack<Node> {
    let lazy_entity = LazyEntity::new();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let focused = signal::from_entity(lazy_entity.clone())
        .has_component::<MenuItemFocused>()
        .dedupe();
    Stack::<Node>::new()
        .lazy_entity(lazy_entity)
        .insert((Pickable::default(), Hoverable))
        .background_color_signal(
            signal::any!(hovered, focused)
                .dedupe()
                .map_bool_in(|| NORMAL_BUTTON.lighter(0.05), || NORMAL_BUTTON)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Px(MENU_ITEM_HEIGHT);
            node.padding = UiRect::axes(Val::Px(BASE_PADDING), Val::Px(BASE_PADDING / 2.));
        })
        .layer(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(label))
                .align(Align::new().left().center_y()),
        )
        .layer(body.align(Align::new().right().center_y()))
}

/// Component to store dropdown selection index
#[derive(Component, Clone, Default, Deref)]
struct DropdownSelectionIndex(Option<usize>);

/// Component marker for dropdown being shown
#[derive(Component, Clone, Default)]
struct DropdownShowing;

/// Component to mark which option index is hovered in dropdown
#[derive(Component, Clone, Default, Deref)]
struct DropdownHoveredIndex(Option<usize>);

/// Component marker for dropdown being clearable
#[derive(Component, Clone, Default)]
struct DropdownClearable;

/// Component to store number of options in dropdown
#[derive(Component, Clone, Default, Deref)]
struct DropdownNumOptions(usize);

struct Dropdown<T: Display + Clone + PartialEq + Send + Sync + 'static> {
    el: El<Node>,
    lazy_entity: LazyEntity,
    options: Vec<T>,
    initial_selection: Option<T>,
    clearable: bool,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> Dropdown<T> {
    fn new(options: Vec<T>) -> Self {
        Self {
            el: El::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            options,
            initial_selection: None,
            clearable: false,
            external_sync_task: None,
            on_change_task: None,
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

    /// Sync the selection from an external signal
    fn selection_signal(mut self, signal: impl Signal<Item = Option<T>> + Clone + Send + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let options = self.options.clone();
        let task = signal
            .dedupe()
            .map(clone!((lazy_entity, options) move |In(selection): In<Option<T>>, mut dropdowns: Query<&mut DropdownSelectionIndex>| {
                if let Ok(mut sel) = dropdowns.get_mut(*lazy_entity) {
                    let new_idx = selection.and_then(|s| options.iter().position(|o| *o == s));
                    if sel.0 != new_idx {
                        sel.0 = new_idx;
                    }
                }
            }))
            .task();
        self.external_sync_task = Some(task);
        self
    }

    /// Called when the selection changes
    fn on_change<M>(mut self, handler: impl IntoSystem<In<Option<T>>, (), M> + Send + Sync + 'static) -> Self {
        let lazy_entity = self.lazy_entity.clone();
        let options = self.options.clone();
        let task = signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity)
            .map_in(deref_copied)
            .map_in(clone!((options) move |idx: Option<usize>| idx.and_then(|i| options.get(i).cloned())))
            .map(handler)
            .task();
        self.on_change_task = Some(task);
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
            external_sync_task,
            on_change_task,
        } = self;

        let show = signal::from_entity(lazy_entity.clone())
            .has_component::<DropdownShowing>()
            .dedupe();
        let opts = options.clone();
        let num_options = options.len();
        let initial_idx = initial_selection.and_then(|s| options.iter().position(|o| *o == s));

        el
        .lazy_entity(lazy_entity.clone())
        .with_builder(move |builder| {
            let mut b = builder
                .insert(DropdownSelectionIndex(initial_idx))
                .insert(DropdownNumOptions(num_options))
                .insert(DropdownHoveredIndex(None));
            if clearable {
                b = b.insert(DropdownClearable);
            }
            if let Some(task) = external_sync_task {
                b = b.hold_tasks([task]);
            }
            if let Some(task) = on_change_task {
                b = b.hold_tasks([task]);
            }
            b
        })
        // .on_click_outside(clone!((lazy_entity) move |_: In<_>, mut commands: Commands| {
        //     commands.entity(*lazy_entity).remove::<DropdownShowing>();
        // }))
        .observe(
            clone!((lazy_entity) move |mut event: On<MenuInputEvent>,
                  showings: Query<&DropdownShowing>,
                  clearables: Query<&DropdownClearable>,
                  mut dropdowns: Query<(&mut DropdownSelectionIndex, &DropdownNumOptions, &mut DropdownHoveredIndex)>,
                  mut commands: Commands| {
                let Ok((mut selection, num_opts, mut hovered_idx)) = dropdowns.get_mut(*lazy_entity) else {
                    return;
                };
                let is_showing = showings.contains(*lazy_entity);
                let is_clearable = clearables.contains(*lazy_entity);

                match event.input {
                    MenuInput::Up | MenuInput::Down => {
                        if is_showing {
                            event.propagate(false);
                            let step = if matches!(event.input, MenuInput::Up) { -1isize } else { 1isize };
                            let mut i = hovered_idx.0.unwrap_or(if step < 0 { num_opts.0 - 1 } else { 0 }) as isize;
                            if hovered_idx.0.is_some() {
                                i = (i + step + num_opts.0 as isize) % num_opts.0 as isize;
                            }
                            // Skip currently selected option
                            if selection.0 == Some(i as usize) {
                                i = (i + step + num_opts.0 as isize) % num_opts.0 as isize;
                            }
                            hovered_idx.0 = Some(i as usize);
                        }
                    }
                    MenuInput::Select => {
                        if let Some(i) = hovered_idx.0 {
                            selection.0 = Some(i);
                            hovered_idx.0 = None;
                        }
                        if is_showing {
                            commands.entity(*lazy_entity).remove::<DropdownShowing>();
                        } else {
                            commands.entity(*lazy_entity).insert(DropdownShowing);
                        }
                    }
                    MenuInput::Back => {
                        if is_showing {
                            event.propagate(false);
                            hovered_idx.0 = None;
                            commands.entity(*lazy_entity).remove::<DropdownShowing>();
                        }
                    }
                    MenuInput::Delete => {
                        if is_clearable {
                            selection.0 = None;
                        }
                    }
                    _ => (),
                }
            }),
        )
        .child(
            Button::new()
                .with_node(|mut node| node.width = Val::Px(300.))
                .body(
                    Stack::<Node>::new()
                        .with_node(|mut node| {
                            node.width = Val::Percent(100.);
                            node.padding = UiRect::horizontal(Val::Px(BASE_PADDING));
                        })
                        .layer({
                            let opts = opts.clone();
                            El::<Text>::new()
                                .align(Align::new().left())
                                .text_font(TextFont::from_font_size(FONT_SIZE))
                                .text_signal(
                                    signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity.clone())
                                        .map_in(deref_copied)
                                        .map_some_in(move |i| opts[i].to_string())
                                        .map_in(Option::unwrap_or_default)
                                        .map_in(Text)
                                        .map_in(Some)
                                )
                        })
                        .layer({
                            let x_button = || signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity.clone())
                                .map_in(deref_copied)
                                .map_in_ref(Option::is_some)
                                .map_true_in(clone!((lazy_entity) move || {
                                    x_button()
                                    .on_click(clone!((lazy_entity) move |_: In<_>, mut selections: Query<&mut DropdownSelectionIndex>| {
                                        if let Ok(mut sel) = selections.get_mut(*lazy_entity) {
                                            sel.0 = None;
                                        }
                                    }))
                                    .observe(|mut click: On<Pointer<Click>>| click.propagate(false))
                                }));
                            let mut el = Row::<Node>::new()
                                .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING))
                                .align(Align::new().right());
                                // TODO: this should work but type inference fails, may need polonius ?
                                // .item_signal(clearable.then(x_button))
                            if clearable {
                                el = el.item_signal(x_button());
                            }
                            el
                                .item(
                                    El::<Text>::new()
                                        .text_font(TextFont::from_font_size(FONT_SIZE))
                                        .text(Text::new("v")))
                        }),
                )
                .on_click(clone!((lazy_entity) move |_: In<_>, world: &mut World| {
                    if world.entity_mut(*lazy_entity).contains::<DropdownShowing>() {
                        world.entity_mut(*lazy_entity).remove::<DropdownShowing>();
                    } else {
                        for entity in world.query_filtered::<Entity, With<DropdownShowing>>().iter(world).collect::<Vec<_>>() {
                            world.entity_mut(entity).remove::<DropdownShowing>();
                        }
                        world.entity_mut(*lazy_entity).insert(DropdownShowing);
                    }
                })),
        )
        .child_signal(
            show.map_true_in(clone!((lazy_entity, opts) move || {
                Column::<Node>::new()
                    .with_node(|mut node| {
                        node.width = Val::Percent(100.);
                        node.top = Val::Percent(100.);
                        node.position_type = PositionType::Absolute;
                    })
                    .items(
                        opts.iter().cloned().enumerate().collect::<Vec<_>>().into_iter()
                        .map(clone!((lazy_entity) move |(i, opt)| {
                            text_button(
                                    signal::once(opt.to_string()),
                                    clone!((lazy_entity) move |_: In<_>, mut dropdowns: Query<&mut DropdownSelectionIndex>, mut commands: Commands| {
                                        dropdowns.get_mut(*lazy_entity).unwrap().0 = Some(i);
                                        commands.entity(*lazy_entity).remove::<DropdownShowing>();
                                    }),
                                )
                                .with_node(|mut node| node.width = Val::Percent(100.))
                                .selected_signal(
                                    signal::from_component_changed::<DropdownHoveredIndex>(lazy_entity.clone())
                                        .map_in(deref_copied)
                                        .eq(Some(i)),
                                )
                        }))
                    )
                    // .items_signal_vec(
                    //     MutableVec::builder().values(opts.iter().cloned().enumerate().collect::<Vec<_>>()).spawn(world)
                    //         .signal_vec()
                    //         .filter_signal(clone!((lazy_entity) move |In((i, _)): In<(usize, T)>| {
                    //             signal::from_component::<DropdownSelectionIndex>(lazy_entity.clone())
                    //                 .map_in(deref_copied)
                    //                 .map_in(move |selected_idx: Option<usize>| selected_idx != Some(i))
                    //                 .dedupe()
                    //         }))
                    //         .map_in(clone!((lazy_entity) move |(i, opt): (usize, T)| {
                    //             text_button(
                    //                 signal::once(opt.to_string()),
                    //                 clone!((lazy_entity) move |_: In<_>, mut dropdowns: Query<&mut DropdownSelectionIndex>, mut commands: Commands| {
                    //                     dropdowns.get_mut(*lazy_entity).unwrap().0 = Some(i);
                    //                     commands.entity(*lazy_entity).remove::<DropdownShowing>();
                    //                 }),
                    //             )
                    //             .with_node(|mut node| node.width = Val::Percent(100.))
                    //             .selected_signal(
                    //                 signal::from_component_changed::<DropdownHoveredIndex>(lazy_entity.clone())
                    //                     .map_in(deref_copied)
                    //                     .eq(Some(i)),
                    //             )
                    //         })),
                    // )
            })),
        )
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> BuilderPassThrough for Dropdown<T> {}

/// Component to store the focusable item's index within its parent container
#[derive(Component, Clone, Default, Deref)]
struct FocusableItemIndex(usize);

fn sub_menu_child_focus_manager<E: Element + BuilderPassThrough>(element: E) -> E {
    let lazy_entity = LazyEntity::new();
    let none_focused = signal::from_component_changed::<FocusedIndex>(lazy_entity.clone())
        .map_in(deref_copied)
        .map_in_ref(Option::is_none);
    element
        .lazy_entity(lazy_entity.clone())
        .insert(FocusedIndex::default())
        .component_signal(none_focused.map_true_in(|| Focused))
        .observe(clone!((lazy_entity) move |event: On<MenuInputEvent>,
            children_query: Query<&Children>,
            mut focused_indices: Query<&mut FocusedIndex>,
            mut commands: Commands| {
                let entity = *lazy_entity;
                let Ok(mut focused_idx) = focused_indices.get_mut(entity) else { return };
                let num_items = children_query.get(entity).map(|c| c.len()).unwrap_or(0);
                if num_items == 0 { return; }
                match event.input {
                    MenuInput::Up | MenuInput::Down => {
                        let step = if matches!(event.input, MenuInput::Up) {
                            -1isize
                        } else {
                            1isize
                        };
                        let i = match focused_idx.0 {
                            Some(cur) => ((cur as isize + step + num_items as isize) % num_items as isize) as usize,
                            None => {
                                if step < 0 {
                                    num_items - 1
                                } else {
                                    0
                                }
                            }
                        };
                        focused_idx.0 = Some(i);
                    }
                    MenuInput::Back => {
                        if focused_idx.0.is_some() {
                            focused_idx.0 = None;
                        } else {
                            commands.insert_resource(ShowSubMenu(None));
                        }
                    }
                    _ => (),
                }
            }
        ))
}

fn focusable_menu_item(label: &str, el: impl Element) -> Stack<Node> {
    let item_entity = LazyEntity::new();
    let focused = signal::eq!(
        signal::from_parent(item_entity.clone())
            .component_changed::<FocusedIndex>()
            .map_in::<Option<usize>, _, _>(deref_copied),
        signal::from_component_changed::<FocusableItemIndex>(item_entity.clone())
            .map_in(deref_copied)
            .map_in(Some)
    )
    .dedupe();
    menu_item(
        label,
        el.with_builder(|builder| builder.component_signal(focused.clone().map_true_in(|| Focused))),
    )
    .lazy_entity(item_entity)
    .component_signal(focused.map_true_in(|| MenuItemFocused))
}

fn audio_menu() -> impl Element {
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "audio menu")
        .apply(sub_menu_child_focus_manager)
        .items(
            [
                (
                    "dropdown",
                    Dropdown::new(options(4))
                        .clearable()
                        .selection_signal(
                            signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings| s.dropdown),
                        )
                        .on_change(|In(v): In<Option<String>>, mut settings: ResMut<AudioSettings>| {
                            settings.dropdown = v;
                        })
                        .type_erase(),
                ),
                (
                    "radio group",
                    RadioGroup::new(options(3))
                        .selection_signal(
                            signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings| s.radio_group),
                        )
                        .on_change(|In(v): In<Option<usize>>, mut settings: ResMut<AudioSettings>| {
                            settings.radio_group = v;
                        })
                        .type_erase(),
                ),
                (
                    "checkbox",
                    Checkbox::new()
                        .checked_signal(
                            signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings| s.checkbox),
                        )
                        .on_change(|In(v): In<bool>, mut settings: ResMut<AudioSettings>| {
                            settings.checkbox = v;
                        })
                        .type_erase(),
                ),
                (
                    "iterable options",
                    IterableOptions::new(options(4))
                        .selection_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|s: AudioSettings| s.iterable_option),
                        )
                        .on_change(|In(v): In<String>, mut settings: ResMut<AudioSettings>| {
                            settings.iterable_option = v;
                        })
                        .type_erase(),
                ),
                (
                    "master volume",
                    Slider::new()
                        // .value_signal(
                        //     signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings|
                        // s.master_volume), )
                        .on_change(|In(v): In<f32>, mut settings: ResMut<AudioSettings>| {
                            settings.master_volume = v;
                        })
                        .type_erase(),
                ),
                (
                    "effect volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings| s.effect_volume),
                        )
                        .on_change(|In(v): In<f32>, mut settings: ResMut<AudioSettings>| {
                            settings.effect_volume = v;
                        })
                        .type_erase(),
                ),
                (
                    "music volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings| s.music_volume),
                        )
                        .on_change(|In(v): In<f32>, mut settings: ResMut<AudioSettings>| {
                            settings.music_volume = v;
                        })
                        .type_erase(),
                ),
                (
                    "voice volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>().map_in(|s: AudioSettings| s.voice_volume),
                        )
                        .on_change(|In(v): In<f32>, mut settings: ResMut<AudioSettings>| {
                            settings.voice_volume = v;
                        })
                        .type_erase(),
                ),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (label, el))| {
                focusable_menu_item(label, el).insert((FocusableItemIndex(i), ZIndex(i32::MAX - i as i32)))
            }),
        )
}

fn graphics_menu() -> impl Element {
    let computed_preset = signal::from_resource::<GraphicsSettings>()
        .map_in(|settings| {
            if settings.texture_quality == settings.shadow_quality && settings.shadow_quality == settings.bloom_quality
            {
                Some(settings.texture_quality)
            } else {
                None
            }
        })
        .dedupe();

    let quality_options = Quality::iter().collect::<Vec<_>>();
    let items = [
        (
            "preset quality",
            Dropdown::new(quality_options.clone())
                .clearable()
                .selection_signal(computed_preset)
                .on_change(
                    |In(quality_option): In<Option<Quality>>, mut settings: ResMut<GraphicsSettings>| {
                        if let Some(quality) = quality_option {
                            settings.texture_quality = quality;
                            settings.shadow_quality = quality;
                            settings.bloom_quality = quality;
                        }
                    },
                ),
        ),
        (
            "texture quality",
            Dropdown::new(quality_options.clone())
                .selection_signal(
                    signal::from_resource_changed::<GraphicsSettings>()
                        .map_in(|settings| settings.texture_quality)
                        .map_in(Some)
                        .dedupe(),
                )
                .on_change(
                    |In(quality_option): In<Option<Quality>>, mut settings: ResMut<GraphicsSettings>| {
                        if let Some(quality) = quality_option {
                            settings.texture_quality = quality;
                        }
                    },
                ),
        ),
        (
            "shadow quality",
            Dropdown::new(quality_options.clone())
                .selection_signal(
                    signal::from_resource::<GraphicsSettings>()
                        .map_in(|settings| settings.shadow_quality)
                        .map_in(Some)
                        .dedupe(),
                )
                .on_change(
                    |In(quality_option): In<Option<Quality>>, mut settings: ResMut<GraphicsSettings>| {
                        if let Some(quality) = quality_option {
                            settings.shadow_quality = quality;
                        }
                    },
                ),
        ),
        (
            "bloom quality",
            Dropdown::new(quality_options.clone())
                .selection_signal(
                    signal::from_resource::<GraphicsSettings>()
                        .map_in(|settings| settings.bloom_quality)
                        .map_in(Some)
                        .dedupe(),
                )
                .on_change(
                    |In(quality_option): In<Option<Quality>>, mut settings: ResMut<GraphicsSettings>| {
                        if let Some(quality) = quality_option {
                            settings.bloom_quality = quality;
                        }
                    },
                ),
        ),
    ];

    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "graphics menu")
        .apply(sub_menu_child_focus_manager)
        .items(items.into_iter().enumerate().map(|(i, (label, el))| {
            focusable_menu_item(label, el).insert((FocusableItemIndex(i), ZIndex(i32::MAX - i as i32)))
        }))
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

fn main_menu() -> impl Element + Clone {
    menu_base(MAIN_MENU_SIDES, MAIN_MENU_SIDES, "main menu")
        .insert(Focused)
        .observe(
            move |event: On<MenuInputEvent>,
                  focuseds: Query<&Focused>,
                  sub_menu_selected: Res<SubMenuSelected>,
                  mut commands: Commands| {
                if !focuseds.contains(event.entity) {
                    return;
                }
                match event.input {
                    MenuInput::Up | MenuInput::Down => {
                        if let Some(cur_sub_menu) = sub_menu_selected.0 {
                            if let Some(i) = SubMenu::iter().position(|sub_menu| cur_sub_menu == sub_menu) {
                                let sub_menus = SubMenu::iter().collect::<Vec<_>>();
                                commands.insert_resource(SubMenuSelected(if matches!(event.input, MenuInput::Down) {
                                    sub_menus.iter().rev().cycle().nth(sub_menus.len() - i).copied()
                                } else {
                                    sub_menus.iter().cycle().nth(i + 1).copied()
                                }));
                            }
                        } else {
                            commands.insert_resource(SubMenuSelected(Some(if matches!(event.input, MenuInput::Up) {
                                SubMenu::iter().next_back().unwrap()
                            } else {
                                SubMenu::iter().next().unwrap()
                            })));
                        }
                    }
                    MenuInput::Select => {
                        if let Some(sub_menu) = sub_menu_selected.0 {
                            commands.insert_resource(ShowSubMenu(Some(sub_menu)));
                        }
                    }
                    MenuInput::Back => {
                        commands.insert_resource(SubMenuSelected(None));
                    }
                    _ => (),
                }
            },
        )
        .with_node(|mut node| node.row_gap = Val::Px(BASE_PADDING * 2.))
        .item(
            Column::<Node>::new()
                .with_node(|mut node| node.row_gap = Val::Px(BASE_PADDING))
                .align_content(Align::center())
                .items(SubMenu::iter().map(|sub_menu| {
                    sub_menu_button(sub_menu).hovered_signal(
                        signal::from_resource_changed::<SubMenuSelected>()
                            .map_in(deref_copied)
                            .eq(Some(sub_menu)),
                    )
                })),
        )
}

fn sub_menu_x(element: impl Element) -> impl Element + Clone {
    Stack::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(SUB_MENU_WIDTH);
            node.height = Val::Px(SUB_MENU_HEIGHT);
            // TODO: without absolute there's some weird bouncing when switching between
            // menus, perhaps due to the layout system having to figure stuff out ?
            node.position_type = PositionType::Absolute;
        })
        .align(Align::center())
        .layer(element.align(Align::center()))
        .layer(
            x_button()
                .on_click(|_: In<_>, mut commands: Commands| {
                    commands.insert_resource(ShowSubMenu(None));
                })
                .align(Align::new().top().right())
                .with_node(|mut node| {
                    node.padding.right = Val::Px(BASE_PADDING);
                    node.padding.top = Val::Px(BASE_PADDING / 2.);
                }),
        )
}

fn menu() -> impl Element {
    Stack::<Node>::new().layer_signal(
        signal::from_resource_changed::<ShowSubMenu>()
            .map_in(deref_copied)
            .map_option_in(
                |sub_menu| {
                    match sub_menu {
                        SubMenu::Audio => audio_menu().left_either(),
                        SubMenu::Graphics => graphics_menu().right_either(),
                    }
                    .apply(sub_menu_x)
                    .left_either()
                },
                || main_menu().right_either(),
            ),
    )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Clone, Copy)]
enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Delete,
}

#[derive(EntityEvent, Clone)]
#[entity_event(propagate, auto_propagate)]
struct MenuInputEvent {
    entity: Entity,
    input: MenuInput,
}

#[derive(Resource)]
struct MenuInputRateLimiter(Timer);

#[derive(Resource)]
struct SliderRateLimiter(Timer);

enum PressedType {
    Pressed,
    JustPressed,
    Neither,
}

fn rate_limited_menu_input(
    pressed_type: PressedType,
    input: MenuInput,
    entity: Entity,
    rate_limiter: &mut Timer,
    time: &Res<Time>,
    commands: &mut Commands,
) -> bool {
    match pressed_type {
        PressedType::Pressed => {
            if rate_limiter.tick(time.delta()).is_finished() {
                commands.trigger(MenuInputEvent { entity, input });
                rate_limiter.reset();
            }
            true
        }
        PressedType::JustPressed => {
            commands.trigger(MenuInputEvent { entity, input });
            rate_limiter.reset();
            true
        }
        PressedType::Neither => false,
    }
}

#[derive(Component)]
struct SliderTag;

fn keyboard_menu_input_events(
    sliders: Query<Entity, With<SliderTag>>,
    focused: Option<Single<Entity, With<Focused>>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut menu_input_rate_limiter: ResMut<MenuInputRateLimiter>,
    mut slider_rate_limiter: ResMut<SliderRateLimiter>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let Some(focused_entity) = focused.map(|f| *f) else {
        return;
    };
    if keys.pressed(KeyCode::ShiftLeft) {
        let pressed_type = if keys.just_pressed(KeyCode::Tab) {
            PressedType::JustPressed
        } else if keys.pressed(KeyCode::Tab) {
            PressedType::Pressed
        } else {
            PressedType::Neither
        };
        let handled = rate_limited_menu_input(
            pressed_type,
            MenuInput::Up,
            focused_entity,
            &mut menu_input_rate_limiter.0,
            &time,
            &mut commands,
        );
        if handled {
            return;
        }
    }
    let slider_focused = sliders.get(focused_entity).is_ok();
    for (key, input) in [
        (KeyCode::ArrowUp, MenuInput::Up),
        (KeyCode::ArrowDown, MenuInput::Down),
        (KeyCode::ArrowLeft, MenuInput::Left),
        (KeyCode::ArrowRight, MenuInput::Right),
        (KeyCode::KeyW, MenuInput::Up),
        (KeyCode::KeyS, MenuInput::Down),
        (KeyCode::KeyA, MenuInput::Left),
        (KeyCode::KeyD, MenuInput::Right),
        (KeyCode::Enter, MenuInput::Select),
        (KeyCode::Escape, MenuInput::Back),
        (KeyCode::Backspace, MenuInput::Back),
        (KeyCode::Tab, MenuInput::Down),
        (KeyCode::Space, MenuInput::Select),
        (KeyCode::Delete, MenuInput::Delete),
    ] {
        let rate_limiter = {
            if slider_focused && matches!(input, MenuInput::Left | MenuInput::Right) {
                &mut slider_rate_limiter.0
            } else {
                &mut menu_input_rate_limiter.0
            }
        };
        let pressed_type = if keys.just_pressed(key) {
            PressedType::JustPressed
        } else if keys.pressed(key) {
            PressedType::Pressed
        } else {
            PressedType::Neither
        };
        rate_limited_menu_input(pressed_type, input, focused_entity, rate_limiter, &time, &mut commands);
    }
}

#[allow(clippy::too_many_arguments)]
fn gamepad_menu_input_events(
    sliders: Query<Entity, With<SliderTag>>,
    focused: Option<Single<Entity, With<Focused>>>,
    gamepads: Query<&Gamepad>,
    mut menu_input_rate_limiter: ResMut<MenuInputRateLimiter>,
    mut slider_rate_limiter: ResMut<SliderRateLimiter>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let Some(focused_entity) = focused.map(|f| *f) else {
        return;
    };
    let slider_focused = sliders.get(focused_entity).is_ok();
    for gamepad in gamepads.iter() {
        for (button, input) in [
            (GamepadButton::DPadUp, MenuInput::Up),
            (GamepadButton::DPadDown, MenuInput::Down),
            (GamepadButton::DPadLeft, MenuInput::Left),
            (GamepadButton::DPadRight, MenuInput::Right),
            (GamepadButton::North, MenuInput::Delete),
            (GamepadButton::South, MenuInput::Select),
            (GamepadButton::East, MenuInput::Back),
        ] {
            let rate_limiter = {
                if slider_focused && matches!(input, MenuInput::Left | MenuInput::Right) {
                    &mut slider_rate_limiter.0
                } else {
                    &mut menu_input_rate_limiter.0
                }
            };
            let pressed_type = if gamepad.pressed(button) {
                PressedType::Pressed
            } else if gamepad.just_pressed(button) {
                PressedType::JustPressed
            } else {
                PressedType::Neither
            };
            rate_limited_menu_input(pressed_type, input, focused_entity, rate_limiter, &time, &mut commands);
        }
    }
}

const MENU_INPUT_RATE_LIMIT: f32 = 0.15;
const SLIDER_RATE_LIMIT: f32 = 0.001;

fn ui_root() -> impl Element {
    El::<Node>::new()
        .ui_root()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(menu())
}
