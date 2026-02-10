//! - Main menu with sub menus for audio and graphics.
//! - Simple buttons for option selection.
//! - Slider for volume.
//! - Dropdown for graphics quality (low/medium/high).
//! - Navigation possible with mouse, keyboard and controller.
//!   - Mouse: Separate styles for hover and press.
//!   - Keyboard/Controller: Separate styles for currently focused element.

mod utils;
use utils::*;

use bevy::{prelude::*, ui::Pressed};
use haalka::{impl_haalka_methods, prelude::*};
use std::fmt::Display;
use strum::{Display as StrumDisplay, EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins(HaalkaPlugin::new().with_jonmo(|jonmo| jonmo.with_schedule::<Update>()))
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
                keyboard_menu_input_events,
                gamepad_menu_input_events,
                virtual_pressed_timer_system,
            ),
        )
        .init_resource::<ShowSubMenu>()
        .insert_resource(AudioSettings {
            master_volume: 100.,
            music_volume: 50.,
            effect_volume: 50.,
            voice_volume: 50.,
            audio_output: AudioOutput::Stereo,
            audio_language: AudioLanguage::English,
        })
        .insert_resource(GraphicsSettings {
            display_mode: DisplayMode::Fullscreen,
            vsync: true,
            resolution: Resolution::R1920x1080,
            texture_quality: Quality::Medium,
            shadow_quality: Quality::Medium,
            anti_aliasing: Some(AntiAliasing::Taa),
        })
        .run();
}

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
const TEXT_BUTTON_WIDTH: f32 = 200.;
const BASE_BORDER_WIDTH: f32 = 5.;
const MENU_ITEM_HEIGHT: f32 = DEFAULT_BUTTON_HEIGHT + BASE_PADDING;
const LIL_BUTTON_SIZE: f32 = 30.;
const SLIDER_SPEED: f32 = 50.;

fn menu() -> impl Element {
    Stack::<Node>::new().layer_signal(
        signal::from_resource_changed::<ShowSubMenu>()
            .dedupe()
            .map_in(deref_copied)
            .map_option_in(
                |sub_menu| {
                    match sub_menu {
                        SubMenu::Graphics => graphics_menu().left_either(),
                        SubMenu::Audio => audio_menu().right_either(),
                    }
                    .apply(closeable_sub_menu)
                    .left_either()
                },
                || main_menu().right_either(),
            ),
    )
}

fn main_menu() -> impl Element + Clone {
    menu_base(MAIN_MENU_SIDES, MAIN_MENU_SIDES, "main menu")
        .with_node(|mut node| node.row_gap = Val::Px(BASE_PADDING * 2.))
        .item(
            Column::<Node>::new()
                .apply(child_focus_manager)
                .with_node(|mut node| node.row_gap = Val::Px(BASE_PADDING))
                .align_content(Align::center())
                .items(SubMenu::iter().enumerate().map(focusable_sub_menu_button)),
        )
}

fn graphics_menu() -> impl Element {
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "graphics menu")
        .apply(child_focus_manager)
        .items(
            [
                (
                    "display mode",
                    Dropdown::new(DisplayMode::iter().collect())
                        .selection_signal(
                            signal::from_resource_changed::<GraphicsSettings>()
                                .map_in(|settings| settings.display_mode)
                                .map_in(Some)
                                .dedupe(),
                        )
                        .on_change(|In(display_mode_option), mut settings: ResMut<GraphicsSettings>| {
                            if let Some(display_mode) = display_mode_option {
                                settings.display_mode = display_mode;
                            }
                        })
                        .type_erase(),
                ),
                (
                    "resolution",
                    Spinner::new(Resolution::iter().collect())
                        .selection_signal(
                            signal::from_resource_changed::<GraphicsSettings>()
                                .map_in(|settings| settings.resolution)
                                .dedupe(),
                        )
                        .on_change(|In(resolution), mut settings: ResMut<GraphicsSettings>| {
                            settings.resolution = resolution;
                        })
                        .type_erase(),
                ),
                (
                    "vsync",
                    Checkbox::new()
                        .checked_signal(
                            signal::from_resource_changed::<GraphicsSettings>()
                                .map_in(|settings| settings.vsync)
                                .dedupe(),
                        )
                        .on_change(|In(vsync), mut settings: ResMut<GraphicsSettings>| {
                            settings.vsync = vsync;
                        })
                        .type_erase(),
                ),
                (
                    "preset quality",
                    Dropdown::new(Quality::iter().collect())
                        .clearable()
                        .selection_signal(
                            signal::from_resource_changed::<GraphicsSettings>()
                                .map_in(|settings| {
                                    if settings.texture_quality == settings.shadow_quality {
                                        Some(settings.texture_quality)
                                    } else {
                                        None
                                    }
                                })
                                .dedupe(),
                        )
                        .on_change(|In(quality_option), mut settings: ResMut<GraphicsSettings>| {
                            if let Some(quality) = quality_option {
                                settings.texture_quality = quality;
                                settings.shadow_quality = quality;
                            }
                        })
                        .type_erase(),
                ),
                (
                    "texture quality",
                    quality_dropdown(
                        |settings| settings.texture_quality,
                        |settings, quality| settings.texture_quality = quality,
                    )
                    .type_erase(),
                ),
                (
                    "shadow quality",
                    quality_dropdown(
                        |settings| settings.shadow_quality,
                        |settings, quality| settings.shadow_quality = quality,
                    )
                    .type_erase(),
                ),
                (
                    "anti-aliasing",
                    RadioGroup::new(AntiAliasing::iter().collect())
                        .selection_signal(
                            signal::from_resource_changed::<GraphicsSettings>()
                                .map_in(|settings| settings.anti_aliasing)
                                .dedupe(),
                        )
                        .on_change(|In(v), mut settings: ResMut<GraphicsSettings>| {
                            settings.anti_aliasing = v;
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

fn audio_menu() -> impl Element {
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "audio menu")
        .apply(child_focus_manager)
        .items(
            [
                (
                    "master volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|settings| settings.master_volume)
                                .dedupe(),
                        )
                        .on_change(|In(volume), mut settings: ResMut<AudioSettings>| {
                            settings.master_volume = volume;
                        })
                        .type_erase(),
                ),
                (
                    "music volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|settings| settings.music_volume)
                                .dedupe(),
                        )
                        .on_change(|In(volume), mut settings: ResMut<AudioSettings>| {
                            settings.music_volume = volume;
                        })
                        .type_erase(),
                ),
                (
                    "effect volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|settings| settings.effect_volume)
                                .dedupe(),
                        )
                        .on_change(|In(volume), mut settings: ResMut<AudioSettings>| {
                            settings.effect_volume = volume;
                        })
                        .type_erase(),
                ),
                (
                    "voice volume",
                    Slider::new()
                        .value_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|settings| settings.voice_volume)
                                .dedupe(),
                        )
                        .on_change(|In(volume), mut settings: ResMut<AudioSettings>| {
                            settings.voice_volume = volume;
                        })
                        .type_erase(),
                ),
                (
                    "audio output",
                    Dropdown::new(AudioOutput::iter().collect::<Vec<_>>())
                        .selection_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|settings| settings.audio_output)
                                .map_in(Some)
                                .dedupe(),
                        )
                        .on_change(|In(output_option), mut settings: ResMut<AudioSettings>| {
                            if let Some(output) = output_option {
                                settings.audio_output = output;
                            }
                        })
                        .type_erase(),
                ),
                (
                    "audio language",
                    Spinner::new(AudioLanguage::iter().collect())
                        .selection_signal(
                            signal::from_resource_changed::<AudioSettings>()
                                .map_in(|settings| settings.audio_language)
                                .dedupe(),
                        )
                        .on_change(|In(language), mut settings: ResMut<AudioSettings>| {
                            settings.audio_language = language;
                        })
                        .text_width(150.)
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

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter)]
enum SubMenu {
    Graphics,
    Audio,
}

/// Resource for audio settings, easily serializable
#[derive(Resource, Clone, PartialEq, Default)]
struct AudioSettings {
    master_volume: f32,
    music_volume: f32,
    effect_volume: f32,
    voice_volume: f32,
    audio_output: AudioOutput,
    audio_language: AudioLanguage,
}

/// Resource for graphics settings, easily serializable
#[derive(Resource, Clone, PartialEq, Default)]
struct GraphicsSettings {
    display_mode: DisplayMode,
    resolution: Resolution,
    vsync: bool,
    texture_quality: Quality,
    shadow_quality: Quality,
    anti_aliasing: Option<AntiAliasing>,
}

#[derive(Resource, Clone, Copy, PartialEq, Default, Deref)]
struct ShowSubMenu(Option<SubMenu>);

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter, Default)]
enum AudioOutput {
    #[default]
    Stereo,
    #[strum(serialize = "Surround 5.1")]
    Surround51,
    #[strum(serialize = "Surround 7.1")]
    Surround71,
    Headphones,
}

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter, Default)]
enum DisplayMode {
    Windowed,
    Borderless,
    #[default]
    Fullscreen,
}

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter, Default)]
enum AudioLanguage {
    #[default]
    English,
    Spanish,
    French,
    Japanese,
    Bengali,
}

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter, Default)]
enum Resolution {
    #[default]
    #[strum(serialize = "1920 x 1080")]
    R1920x1080,
    #[strum(serialize = "2560 x 1440")]
    R2560x1440,
    #[strum(serialize = "3840 x 2160")]
    R3840x2160,
}

#[derive(Clone, Copy, PartialEq, StrumDisplay, EnumIter, Default)]
enum AntiAliasing {
    Fxaa,
    #[default]
    Taa,
    Smaa,
}

/// Component to mark a button as selected (for external control)
#[derive(Component, Clone, Default)]
struct Selected;

/// Component for virtual hover state (keyboard/gamepad navigation)
#[derive(Component, Clone, Default)]
struct VirtualHovered;

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
        z_index: ZIndex,
        visibility: Visibility,
    }
}

impl Button {
    fn new() -> Self {
        let lazy_entity = LazyEntity::new();
        let pressed = signal::from_entity(lazy_entity.clone())
            .has_component::<Pressed>()
            .dedupe();
        let virtual_pressed = signal::from_entity(lazy_entity.clone())
            .has_component::<VirtualPressed>()
            .dedupe();
        let dragged = signal::from_entity(lazy_entity.clone())
            .has_component::<Dragged>()
            .dedupe();
        let mouse_hovered = signal::from_entity(lazy_entity.clone())
            .has_component::<Hovered>()
            .dedupe();
        let virtual_hovered = signal::from_entity(lazy_entity.clone())
            .has_component::<VirtualHovered>()
            .dedupe();
        let selected = signal::from_entity(lazy_entity.clone())
            .has_component::<Selected>()
            .dedupe();
        let selected_hovered = signal::zip!(
            signal::any!(selected.clone(), pressed, virtual_pressed, dragged).dedupe(),
            signal::any!(mouse_hovered, virtual_hovered).dedupe()
        )
        .dedupe();

        Self {
            el: {
                El::<Node>::new()
                    .lazy_entity(lazy_entity.clone())
                    .insert((Pickable::default(), Hoverable, Pressable, Draggable))
                    .with_node(|mut node| {
                        node.height = Val::Px(DEFAULT_BUTTON_HEIGHT);
                        node.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                    })
                    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                    .on_dragged_change(
                        |In((_, dragged_data)): In<(Entity, DragData)>, mut commands: Commands| {
                            if dragged_data.dragged {
                                commands.insert_resource(CursorableDisabled);
                            } else {
                                commands.remove_resource::<CursorableDisabled>();
                            }
                        },
                    )
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

    fn selected_signal(mut self, selected: impl Signal<Item = bool> + Clone + 'static) -> Self {
        self.el = self
            .el
            .component_signal(selected.map_true_in(|| Selected).schedule::<Update>());
        self
    }

    fn hovered_signal(mut self, hovered: impl Signal<Item = bool> + Clone + 'static) -> Self {
        self.el = self.el.component_signal(hovered.map_true_in(|| VirtualHovered));
        self
    }
}

fn text_button(text_signal: impl Signal<Item = String> + Clone + 'static) -> Button {
    Button::new()
        .body(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(text_signal.map_in(Text).map_in(Some)),
        )
        .with_node(|mut node| node.width = Val::Px(TEXT_BUTTON_WIDTH))
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
        .observe(move |event: On<MenuInputEvent>, mut commands: Commands| {
            if matches!(event.input, MenuInput::Select) {
                commands.insert_resource(ShowSubMenu(Some(sub_menu)));
            }
        })
        .with_node(|mut node| node.width = Val::Px(200.))
}

fn focusable_sub_menu_button((index, sub_menu): (usize, SubMenu)) -> El<Node> {
    let lazy_entity = LazyEntity::new();
    let focused = signal::from_parent(lazy_entity.clone())
        .component_changed::<FocusedIndex>()
        .map_in::<Option<usize>, _, _>(deref_copied)
        .eq(Some(index))
        .dedupe();
    El::<Node>::new()
        .lazy_entity(lazy_entity)
        .insert(FocusableItemIndex(index))
        .child(
            sub_menu_button(sub_menu)
                .hovered_signal(focused.clone())
                .with_builder(|builder| builder.component_signal(focused.clone().map_true_in(|| Focused))),
        )
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

fn lil_button() -> Button {
    Button::new().with_node(|mut node| {
        node.width = Val::Px(LIL_BUTTON_SIZE);
        node.height = Val::Px(LIL_BUTTON_SIZE);
    })
}

/// Marker component for the currently focused widget (receives MenuInputEvents)
#[derive(Component, Clone, Default)]
struct Focused;

/// Component on ancestor containers that determines which child index is focused
#[derive(Component, Clone, Default, Deref, DerefMut)]
struct FocusedIndex(Option<usize>);

/// Marker component for checked checkboxes
#[derive(Component, Clone, Default)]
struct Checked;

fn checkbox_input_observer(event: On<MenuInputEvent>, checkeds: Query<&Checked>, mut commands: Commands) {
    let entity = event.entity;
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
}

struct Checkbox {
    el: El<Node>,
    lazy_entity: LazyEntity,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl Checkbox {
    fn new() -> Self {
        Self {
            el: El::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn checked_signal(mut self, signal: impl Signal<Item = bool> + Clone + 'static) -> Self {
        let task = signal
            .map(
                clone!((self.lazy_entity => lazy_entity) move |In(checked), checkeds: Query<&Checked>, mut commands: Commands| {
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

    fn on_change<M>(mut self, handler: impl IntoSystem<In<bool>, (), M> + Send + Sync + 'static) -> Self {
        let task = signal::from_entity(self.lazy_entity.clone())
            .has_component::<Checked>()
            // otherwise, view will thrash between external value signal and widget-internal default, prioritizes
            // external sync
            .skip(1)
            .dedupe()
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
            external_sync_task,
            on_change_task,
        } = self;

        lil_button()
            .with_builder(clone!((lazy_entity) move |builder| {
                let mut b = builder.lazy_entity(lazy_entity.clone());
                if let Some(task) = external_sync_task {
                    b = b.hold_tasks([task]);
                }
                if let Some(task) = on_change_task {
                    b = b.hold_tasks([task]);
                }
                b
            }))
            .observe(checkbox_input_observer)
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

/// Wrapping index navigation helper, used for keyboard navigation in menus and dropdowns.
/// Returns the next index when navigating up/down through a list of `count` items.
/// - `current`: The current index (None if nothing selected)
/// - `up`: True for up/previous, false for down/next
/// - `count`: Total number of items
/// - `skip`: Optional index to skip (e.g., skip the currently selected item in a dropdown)
fn wrap_index(current: Option<usize>, up: bool, count: usize, skip: Option<usize>) -> usize {
    let step: isize = if up { -1 } else { 1 };
    let mut i = match current {
        Some(cur) => ((cur as isize + step + count as isize) % count as isize) as usize,
        None => {
            if up {
                count - 1
            } else {
                0
            }
        }
    };
    if skip == Some(i) {
        i = ((i as isize + step + count as isize) % count as isize) as usize;
    }
    i
}

#[derive(Clone, Copy, EnumIter, PartialEq, StrumDisplay, Default)]
enum Quality {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

/// Component to store radio group selection
#[derive(Component, Clone, Default, Deref)]
struct RadioSelection(Option<usize>);

/// Component to store the radio group's option count
#[derive(Component, Clone, Deref)]
struct RadioOptions(usize);

fn radio_input_observer(event: On<MenuInputEvent>, mut selections: Query<(&mut RadioSelection, &RadioOptions)>) {
    let (mut selection_option, num_options) = selections.get_mut(event.entity).unwrap();
    match event.input {
        MenuInput::Left | MenuInput::Right => {
            selection_option.0 = Some(wrap_index(
                selection_option.0,
                matches!(event.input, MenuInput::Left),
                num_options.0,
                None,
            ));
        }
        MenuInput::Delete => {
            selection_option.0 = None;
        }
        _ => (),
    }
}

struct RadioGroup<T: Display + Clone + PartialEq + Send + Sync + 'static> {
    el: Row<Node>,
    lazy_entity: LazyEntity,
    options: Vec<T>,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> RadioGroup<T> {
    fn new(options: Vec<T>) -> Self {
        Self {
            el: Row::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            options,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn selection_signal(mut self, signal: impl Signal<Item = Option<T>> + Clone + 'static) -> Self {
        let task = signal
            .map(
                clone!((self.lazy_entity => lazy_entity, self.options => options) move |In(selection_option): In<Option<T>>,
                            mut selections: Query<&mut RadioSelection>| {
                    let mut selected_option = selections.get_mut(*lazy_entity).unwrap();
                    let new_i_option = selection_option.and_then(|selected| options.iter().position(|option| *option == selected));
                    if selected_option.0 != new_i_option {
                        selected_option.0 = new_i_option;
                    }
                }),
            )
            .task();
        self.external_sync_task = Some(task);
        self
    }

    fn on_change<M>(mut self, handler: impl IntoSystem<In<Option<T>>, (), M> + Send + Sync + 'static) -> Self {
        let task = signal::from_component_changed::<RadioSelection>(self.lazy_entity.clone())
            .map_in(deref_copied)
            .map_in(clone!((self.options => options) move |i: Option<usize>| i.and_then(|i| options.get(i).cloned())))
            // otherwise, view will thrash between external value signal and widget-internal default, prioritizes
            // external sync
            .skip(1)
            .dedupe()
            .map(handler)
            .task();
        self.on_change_task = Some(task);
        self
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> ElementWrapper for RadioGroup<T> {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            lazy_entity,
            options,
            external_sync_task,
            on_change_task,
        } = self;

        let options = options.clone();
        let num_options = options.len();
        el.with_builder(clone!((lazy_entity) move |builder| {
            let mut b = builder
                .lazy_entity(lazy_entity.clone())
                .insert(RadioSelection::default())
                .insert(RadioOptions(num_options));
            if let Some(task) = external_sync_task {
                b = b.hold_tasks([task]);
            }
            if let Some(task) = on_change_task {
                b = b.hold_tasks([task]);
            }
            b
        }))
        .observe(radio_input_observer)
        .items(
            options
                .into_iter()
                .enumerate()
                .map(clone!((lazy_entity) move |(i, option)| {
                    text_button(signal::once(option.to_string()))
                        .on_click(clone!((lazy_entity) move |_: In<_>, mut selections: Query<&mut RadioSelection>| {
                            let mut selection_option = selections.get_mut(*lazy_entity).unwrap();
                            if selection_option.0 == Some(i) {
                                selection_option.0 = None;
                            } else {
                                selection_option.0 = Some(i);
                            }
                        }))
                        .selected_signal(
                            signal::from_component_changed::<RadioSelection>(lazy_entity.clone())
                                .map_in(deref_copied)
                                .eq(Some(i))
                                .dedupe(),
                        )
                })),
        )
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> BuilderPassThrough for RadioGroup<T> {}

#[derive(Clone, Copy)]
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

/// Component to store spinner selection index
#[derive(Component, Clone, Copy, Default, Deref, DerefMut)]
struct SpinnerSelection(usize);

/// Component to store the spinner option count
#[derive(Component, Clone, Deref)]
struct SpinnerOptions(usize);

/// Visual press flash that self-destructs after a timer
#[derive(Component)]
struct VirtualPressed(Timer);

impl VirtualPressed {
    fn new() -> Self {
        Self(Timer::from_seconds(FLASH_MS / 1000., TimerMode::Once))
    }
}

/// Component to store spinner left/right button entities
#[derive(Component, Clone, Default)]
struct SpinnerButtons {
    left: Option<Entity>,
    right: Option<Entity>,
}

fn register_as_spinner_button<E: Element>(direction: LeftRight) -> impl FnOnce(E) -> E {
    move |element: E| {
        element.with_builder(move |builder| {
            builder.on_spawn_with_system(
                move |In(entity), child_ofs: Query<&ChildOf>, mut buttons: Query<&mut SpinnerButtons>| {
                    let parent = child_ofs.get(entity).unwrap().0;
                    let mut buttons = buttons.get_mut(parent).unwrap();
                    match direction {
                        LeftRight::Left => buttons.left = Some(entity),
                        LeftRight::Right => buttons.right = Some(entity),
                    }
                },
            )
        })
    }
}

const FLASH_MS: f32 = 50.;

fn virtual_pressed_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timers: Query<(Entity, &mut VirtualPressed)>,
) {
    for (entity, mut vp) in timers.iter_mut() {
        vp.0.tick(time.delta());
        if vp.0.is_finished() {
            commands.entity(entity).remove::<VirtualPressed>();
        }
    }
}

fn spinner_input_observer(
    event: On<MenuInputEvent>,
    mut spinners: Query<(&mut SpinnerSelection, &SpinnerOptions, &SpinnerButtons)>,
    mut commands: Commands,
) {
    let (mut selection, num_options, buttons) = spinners.get_mut(event.entity).unwrap();
    if num_options.0 == 0 {
        return;
    }
    match event.input {
        MenuInput::Left | MenuInput::Right => {
            let up = matches!(event.input, MenuInput::Left);
            let button_entity = if up {
                buttons.left.unwrap()
            } else {
                buttons.right.unwrap()
            };
            commands.entity(button_entity).insert(VirtualPressed::new());
            selection.0 = wrap_index(Some(selection.0), up, num_options.0, None);
        }
        _ => (),
    }
}

struct Spinner<T: Display + Clone + PartialEq + Send + Sync + 'static> {
    el: Row<Node>,
    lazy_entity: LazyEntity,
    options: Vec<T>,
    text_width: Option<f32>,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> Spinner<T> {
    fn new(options: Vec<T>) -> Self {
        Self {
            el: Row::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            options,
            text_width: None,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn text_width(mut self, width: f32) -> Self {
        self.text_width = Some(width);
        self
    }

    fn selection_signal(mut self, signal: impl Signal<Item = T> + Clone + 'static) -> Self {
        let task = signal
            .map(
                clone!((self.lazy_entity => lazy_entity, self.options => options) move |In(selection): In<T>,
                       mut spinners: Query<&mut SpinnerSelection>| {
                    let mut selected = spinners.get_mut(*lazy_entity).unwrap();
                    if let Some(i) = options.iter().position(|option| *option == selection)
                        && selected.0 != i {
                            selected.0 = i;
                        }
                }),
            )
            .task();
        self.external_sync_task = Some(task);
        self
    }

    fn on_change<M>(mut self, handler: impl IntoSystem<In<T>, (), M> + Send + Sync + 'static) -> Self {
        let task = signal::from_component_changed::<SpinnerSelection>(self.lazy_entity.clone())
            .map_in(deref_copied)
            // otherwise, view will thrash between external value signal and widget-internal default, prioritizes
            // external sync
            .skip(1)
            .dedupe()
            .map_in(clone!((self.options => options) move |i: usize| options.get(i).cloned().unwrap()))
            .map(handler)
            .task();
        self.on_change_task = Some(task);
        self
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> ElementWrapper for Spinner<T> {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            lazy_entity,
            options,
            text_width,
            external_sync_task,
            on_change_task,
        } = self;

        let options = options.clone();
        let num_options = options.len();
        el.with_builder(clone!((lazy_entity) move |builder| {
            let mut b = builder
                .lazy_entity(lazy_entity.clone())
                .insert(SpinnerSelection::default())
                .insert(SpinnerOptions(num_options))
                .insert(SpinnerButtons::default());
            if let Some(task) = external_sync_task {
                b = b.hold_tasks([task]);
            }
            if let Some(task) = on_change_task {
                b = b.hold_tasks([task]);
            }
            b
        }))
        .observe(spinner_input_observer)
        .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING * 2.))
        .item(
            lil_button()
                .apply(register_as_spinner_button(LeftRight::Left))
                .on_click(
                    clone!((lazy_entity) move |_: In<_>, mut spinners: Query<&mut SpinnerSelection>| {
                        let mut selection = spinners.get_mut(*lazy_entity).unwrap();
                        if num_options > 0 {
                            selection.0 = if selection.0 == 0 { num_options - 1 } else { selection.0 - 1 };
                        }
                    }),
                )
                .body(arrow_text(LeftRight::Left)),
        )
        .item({
            let options = options.clone();
            let mut text_el = El::<Text>::new()
                .with_node(|mut node| node.top = Val::Px(2.))
                .text_layout(TextLayout::new_with_justify(Justify::Center))
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(
                    signal::from_component_changed::<SpinnerSelection>(lazy_entity.clone())
                        .map_in(deref_copied)
                        .map_in(move |i: usize| options.get(i).map(|option| option.to_string()).unwrap_or_default())
                        .map_in(Text)
                        .map_in(Some),
                );
            if let Some(width) = text_width {
                text_el = text_el.with_node(move |mut node| node.width = Val::Px(width));
            }
            text_el
        })
        .item(
            lil_button()
                .apply(register_as_spinner_button(LeftRight::Right))
                .on_click(
                    clone!((lazy_entity) move |_: In<_>, mut spinners: Query<&mut SpinnerSelection>| {
                        let mut selection = spinners.get_mut(*lazy_entity).unwrap();
                        if num_options > 0 {
                            selection.0 = (selection.0 + 1) % num_options;
                        }
                    }),
                )
                .body(arrow_text(LeftRight::Right)),
        )
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> BuilderPassThrough for Spinner<T> {}

/// Component to store slider value with a generation counter.
/// The generation is bumped on every user interaction, allowing the external sync
/// signal to detect and skip stale echo values without marker components.
///
/// TODO: mention this pattern for bidirectional syncing in the docs
#[derive(Component, Clone)]
struct SliderValue {
    value: f32,
    generation: u64,
}

const SLIDER_WIDTH: f32 = 400.;
const SLIDER_PADDING: f32 = 5.;
const SLIDER_MAX: f32 = SLIDER_WIDTH - SLIDER_PADDING - LIL_BUTTON_SIZE - BASE_BORDER_WIDTH;

struct Slider {
    el: Row<Node>,
    lazy_entity: LazyEntity,
    external_sync_task: Option<Box<dyn SignalTask>>,
    on_change_task: Option<Box<dyn SignalTask>>,
}

impl Slider {
    fn new() -> Self {
        Self {
            el: Row::<Node>::new(),
            lazy_entity: LazyEntity::new(),
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn value_signal(mut self, signal: impl Signal<Item = f32> + Clone + 'static) -> Self {
        let task = signal
            .map(clone!((self.lazy_entity => lazy_entity) move |In(value): In<f32>,
                   mut sliders: Query<&mut SliderValue>,
                   mut last_synced_gen: Local<u64>| {
                let Ok(mut slider) = sliders.get_mut(*lazy_entity) else {
                    return;
                };
                if slider.generation != *last_synced_gen {
                    // User has interacted since our last write; skip the stale echo
                    // but acknowledge the new generation
                    *last_synced_gen = slider.generation;
                    return;
                }
                if (slider.value - value).abs() > 0.01 {
                    slider.value = value;
                    // Don't bump generation, this is an external sync, not user input
                }
            }))
            .task();
        self.external_sync_task = Some(task);
        self
    }

    fn on_change<M>(mut self, handler: impl IntoSystem<In<f32>, (), M> + Send + Sync + 'static) -> Self {
        let task = signal::from_component_changed::<SliderValue>(self.lazy_entity.clone())
            .map_in(|slider| slider.value)
            // otherwise, view will thrash between external value signal and widget-internal default, prioritizes
            // external sync
            .skip(1)
            .dedupe()
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
            external_sync_task,
            on_change_task,
        } = self;

        let max = SLIDER_MAX;

        el
            .insert(SliderTag)
            .with_builder(clone!((lazy_entity) move |builder| {
                let mut b = builder
                    .lazy_entity(lazy_entity.clone())
                    .insert(SliderValue { value: 0., generation: 0 });
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
                      time: Res<Time>,
                      mut sliders: Query<&mut SliderValue>| {
                    let mut slider = sliders.get_mut(*lazy_entity).unwrap();
                    match event.input {
                        MenuInput::Left | MenuInput::Right => {
                            let dir = if matches!(event.input, MenuInput::Left) { -1. } else { 1. };
                            slider.value = (slider.value + dir * SLIDER_SPEED * time.delta_secs()).clamp(0., 100.);
                            slider.generation += 1;
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
                            .map_in(|slider| slider.value)
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
                        lil_button()
                            .selected_signal(
                                signal::from_entity(knob_entity.clone())
                                    .has_component::<Pressed>()
                                    .dedupe(),
                            )
                            .into_el() // we need lower level access now
                            .lazy_entity(knob_entity.clone())
                            .insert(Pickable::default())
                            .on_signal_with_node(
                                signal::from_component_changed::<SliderValue>(lazy_entity.clone())
                                    .map_in(|slider| slider.value),
                                move |mut node, value| node.left = Val::Px(value / 100. * max),
                            )
                            .align(Align::new().center_y())
                            .on_dragged(clone!((lazy_entity) move |In((_, drag)): In<(Entity, DragData)>, mut sliders: Query<&mut SliderValue>| {
                                if drag.dragged {
                                    let mut slider = sliders.get_mut(*lazy_entity).unwrap();
                                    slider.value = (slider.value + drag.delta.x / max * 100.).clamp(0., 100.);
                                    slider.generation += 1;
                                }
                            }))
                    }),
            )
    }
}

impl BuilderPassThrough for Slider {}

/// Marker component for menu items that are hovered
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

/// Marker component for dropdown being shown
#[derive(Component, Clone, Default)]
struct DropdownShowing;

/// Component to mark which option index is hovered in dropdown
#[derive(Component, Clone, Default, Deref)]
struct DropdownHoveredIndex(Option<usize>);

/// Marker component for dropdown being clearable
#[derive(Component, Clone, Default)]
struct DropdownClearable;

/// Component to store number of options in dropdown
#[derive(Component, Clone, Default, Deref)]
struct DropdownNumOptions(usize);

fn dropdown_input_observer(
    mut event: On<MenuInputEvent>,
    showings: Query<&DropdownShowing>,
    clearables: Query<&DropdownClearable>,
    mut dropdowns: Query<(
        &mut DropdownSelectionIndex,
        &DropdownNumOptions,
        &mut DropdownHoveredIndex,
    )>,
    mut commands: Commands,
) {
    let entity = event.entity;
    let (mut selection, num_options, mut hovered_i) = dropdowns.get_mut(entity).unwrap();
    let is_showing = showings.contains(entity);
    let is_clearable = clearables.contains(entity);

    match event.input {
        MenuInput::Up | MenuInput::Down => {
            if is_showing {
                event.propagate(false);
                hovered_i.0 = Some(wrap_index(
                    hovered_i.0,
                    matches!(event.input, MenuInput::Up),
                    num_options.0,
                    selection.0,
                ));
            }
        }
        MenuInput::Select => {
            if let Some(i) = hovered_i.0 {
                selection.0 = Some(i);
                hovered_i.0 = None;
            }
            if is_showing {
                commands.entity(entity).remove::<DropdownShowing>();
            } else {
                commands.entity(entity).insert(DropdownShowing);
            }
        }
        MenuInput::Back => {
            if is_showing {
                event.propagate(false);
                hovered_i.0 = None;
                commands.entity(entity).remove::<DropdownShowing>();
            }
        }
        MenuInput::Delete => {
            if is_clearable {
                selection.0 = None;
            }
        }
        _ => (),
    }
}

struct Dropdown<T: Display + Clone + PartialEq + Send + Sync + 'static> {
    el: El<Node>,
    lazy_entity: LazyEntity,
    options: Vec<T>,
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
            clearable: false,
            external_sync_task: None,
            on_change_task: None,
        }
    }

    fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    fn selection_signal(mut self, signal: impl Signal<Item = Option<T>> + Clone + 'static) -> Self {
        let task = signal
            .map(
                clone!((self.lazy_entity => lazy_entity, self.options => options) move |In(selection_option): In<Option<T>>,
                            mut dropdowns: Query<&mut DropdownSelectionIndex>| {
                    let mut selected_option = dropdowns.get_mut(*lazy_entity).unwrap();
                    let new_i = selection_option.and_then(|selection| options.iter().position(|option| *option == selection));
                    if selected_option.0 != new_i {
                        selected_option.0 = new_i;
                    }
                }),
            )
            .task();
        self.external_sync_task = Some(task);
        self
    }

    fn on_change<M>(mut self, handler: impl IntoSystem<In<Option<T>>, (), M> + Send + Sync + 'static) -> Self {
        let task = signal::from_component_changed::<DropdownSelectionIndex>(self.lazy_entity.clone())
            .map_in(deref_copied)
            .map_in(clone!((self.options => options) move |i_option: Option<usize>| i_option.and_then(|i| options.get(i).cloned())))
            .skip(1) // otherwise, view will thrash between external value signal and widget-internal default, prioritizes external sync
            .dedupe()
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
            clearable,
            external_sync_task,
            on_change_task,
        } = self;

        let show = signal::from_entity(lazy_entity.clone())
            .has_component::<DropdownShowing>()
            .dedupe();
        let options = options.clone();
        let num_options = options.len();

        el
        .lazy_entity(lazy_entity.clone())
        .with_builder(move |builder| {
            let mut b = builder
                .insert(DropdownSelectionIndex(None))
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
        .observe(dropdown_input_observer)
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
                            let options = options.clone();
                            El::<Text>::new()
                                .align(Align::new().left())
                                .text_font(TextFont::from_font_size(FONT_SIZE))
                                .text_signal(
                                    signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity.clone())
                                        .map_in(deref_copied)
                                        .map_some_in(move |i| options[i].to_string())
                                        .map_in(Option::unwrap_or_default)
                                        .map_in(Text)
                                        .map_in(Some)
                                )
                        })
                        .layer({
                            let x_button_signal_function = || signal::from_component_changed::<DropdownSelectionIndex>(lazy_entity.clone())
                                .map_in(deref_copied)
                                .map_in_ref(Option::is_some)
                                .map_true_in(clone!((lazy_entity) move || {
                                    x_button()
                                    .on_click(clone!((lazy_entity) move |_: In<_>, mut selections: Query<&mut DropdownSelectionIndex>| {
                                        selections.get_mut(*lazy_entity).unwrap().0 = None;
                                    }))
                                    .observe(|mut click: On<Pointer<Click>>| click.propagate(false))
                                }));
                            let mut el = Row::<Node>::new()
                                .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING))
                                .align(Align::new().right());
                                // TODO: this should work but type inference fails, may need polonius ?
                                // .item_signal(clearable.then(x_button_signal_function))
                            if clearable {
                                el = el.item_signal(x_button_signal_function());
                            }
                            el
                                .item(
                                    El::<Text>::new()
                                        .text_font(TextFont::from_font_size(FONT_SIZE))
                                        .text(Text::new("v")))
                        }),
                )
                .on_click(clone!((lazy_entity) move |_: In<_>, showings: Query<(), With<DropdownShowing>>, mut commands: Commands| {
                    let entity = *lazy_entity;
                    if showings.contains(entity) {
                        commands.entity(entity).remove::<DropdownShowing>();
                    } else {
                        commands.entity(entity).insert(DropdownShowing);
                    }
                }))
        )
        .child_signal(
            show.map_true_in(clone!((lazy_entity, options) move || {
                Column::<Node>::new()
                    .with_node(|mut node| {
                        node.width = Val::Percent(100.);
                        node.top = Val::Percent(100.);
                        node.position_type = PositionType::Absolute;
                    })
                    .on_click_outside(clone!((lazy_entity) move |_: In<_>, mut commands: Commands| {
                        commands.entity(*lazy_entity).remove::<DropdownShowing>();
                    }))
                    .items_signal_vec(
                        signal::once(options.iter().cloned().enumerate().collect())
                            .to_signal_vec()
                            .filter_signal(clone!((lazy_entity) move |In((i, _))| {
                                signal::from_component::<DropdownSelectionIndex>(lazy_entity.clone())
                                    .map_in(deref_copied)
                                    .map_in(move |selected_option: Option<usize>| selected_option != Some(i))
                                    .dedupe()
                                    .schedule::<Update>()
                            }))
                            .map_in(clone!((lazy_entity) move |(i, option)| {
                                text_button(signal::once(option.to_string()))
                                    .on_click(clone!((lazy_entity) move |_: In<_>, mut dropdowns: Query<&mut DropdownSelectionIndex>, mut commands: Commands| {
                                        dropdowns.get_mut(*lazy_entity).unwrap().0 = Some(i);
                                        commands.entity(*lazy_entity).remove::<DropdownShowing>();
                                    }))
                                    .with_node(|mut node| node.width = Val::Percent(100.))
                                    .selected_signal(
                                        signal::from_component_changed::<DropdownHoveredIndex>(lazy_entity.clone())
                                            .map_in(deref_copied)
                                            .eq(Some(i))
                                    )
                            }))
                            .schedule::<Update>(),
                    )
            }))
            .schedule::<Update>(),
        )
    }
}

impl<T: Display + Clone + PartialEq + Send + Sync + 'static> BuilderPassThrough for Dropdown<T> {}

/// Component to store the focusable item's index within its parent container
#[derive(Component, Clone, Default, Deref)]
struct FocusableItemIndex(usize);

fn focus_navigation_observer(
    event: On<MenuInputEvent>,
    children_query: Query<&Children>,
    focusable_items: Query<&FocusableItemIndex>,
    mut focused_indices: Query<&mut FocusedIndex>,
    mut commands: Commands,
) {
    let entity = event.entity;
    let mut focused_i = focused_indices.get_mut(entity).unwrap();
    let Some(children) = children_query.get(entity).ok() else {
        return;
    };
    let num_items = children.iter().filter(|c| focusable_items.contains(*c)).count();
    if num_items == 0 {
        return;
    }
    match event.input {
        MenuInput::Up | MenuInput::Down => {
            focused_i.0 = Some(wrap_index(
                focused_i.0,
                matches!(event.input, MenuInput::Up),
                num_items,
                None,
            ));
        }
        MenuInput::Back => {
            if focused_i.0.is_some() {
                focused_i.0 = None;
            } else {
                commands.insert_resource(ShowSubMenu(None));
            }
        }
        _ => (),
    }
}

fn child_focus_manager<E: Element + BuilderPassThrough>(element: E) -> E {
    let lazy_entity = LazyEntity::new();
    let none_focused = signal::from_component_changed::<FocusedIndex>(lazy_entity.clone())
        .map_in(deref_copied)
        .map_in_ref(Option::is_none);
    element
        .lazy_entity(lazy_entity.clone())
        .insert((FocusedIndex::default(), Focused))
        .component_signal(none_focused.map_true_in(|| Focused))
        .observe(focus_navigation_observer)
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

fn quality_dropdown(
    get: fn(&GraphicsSettings) -> Quality,
    set: fn(&mut GraphicsSettings, Quality),
) -> Dropdown<Quality> {
    Dropdown::new(Quality::iter().collect())
        .selection_signal(
            signal::from_resource_changed::<GraphicsSettings>()
                .map_in(move |settings| get(&settings))
                .map_in(Some)
                .dedupe(),
        )
        .on_change(move |In(quality_option), mut settings: ResMut<GraphicsSettings>| {
            if let Some(quality) = quality_option {
                set(&mut settings, quality);
            }
        })
}

fn x_button() -> El<Node> {
    let lazy_entity = LazyEntity::new();
    El::<Node>::new()
        .lazy_entity(lazy_entity.clone())
        .background_color(BackgroundColor(Color::NONE))
        .insert((Pickable::default(), Hoverable))
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new("x"))
                .text_color_signal(
                    signal::from_entity(lazy_entity)
                        .has_component::<Hovered>()
                        .map_bool_in(|| bevy::color::palettes::basic::RED.into(), || TEXT_COLOR)
                        .map_in(TextColor)
                        .map_in(Some),
                ),
        )
}

fn closeable_sub_menu(element: impl Element) -> impl Element + Clone {
    Stack::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(SUB_MENU_WIDTH);
            node.height = Val::Px(SUB_MENU_HEIGHT);
        })
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

#[derive(Component)]
struct SliderTag;

fn keyboard_menu_input_events(
    sliders: Query<Entity, With<SliderTag>>,
    focused_option: Option<Single<Entity, With<Focused>>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let Some(focused_entity) = focused_option.map(|focused| *focused) else {
        return;
    };
    if keys.pressed(KeyCode::ShiftLeft) && keys.just_pressed(KeyCode::Tab) {
        commands.trigger(MenuInputEvent {
            entity: focused_entity,
            input: MenuInput::Up,
        });
        return;
    }
    let slider_focused = sliders.contains(focused_entity);
    for key in keys.get_just_pressed() {
        let input = match key {
            KeyCode::ArrowUp | KeyCode::KeyW => MenuInput::Up,
            KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => MenuInput::Down,
            // Skip left/right for sliders since they're handled in get_pressed loop
            KeyCode::ArrowLeft | KeyCode::KeyA if !slider_focused => MenuInput::Left,
            KeyCode::ArrowRight | KeyCode::KeyD if !slider_focused => MenuInput::Right,
            KeyCode::Enter | KeyCode::Space => MenuInput::Select,
            KeyCode::Escape | KeyCode::Backspace => MenuInput::Back,
            KeyCode::Delete => MenuInput::Delete,
            _ => continue,
        };
        commands.trigger(MenuInputEvent {
            entity: focused_entity,
            input,
        });
    }
    if slider_focused {
        for key in keys.get_pressed() {
            let input = match key {
                KeyCode::ArrowLeft | KeyCode::KeyA => MenuInput::Left,
                KeyCode::ArrowRight | KeyCode::KeyD => MenuInput::Right,
                _ => continue,
            };
            commands.trigger(MenuInputEvent {
                entity: focused_entity,
                input,
            });
        }
    }
}

fn gamepad_menu_input_events(
    sliders: Query<Entity, With<SliderTag>>,
    focused_option: Option<Single<Entity, With<Focused>>>,
    gamepads: Query<&Gamepad>,
    mut commands: Commands,
) {
    let Some(focused_entity) = focused_option.map(|focused| *focused) else {
        return;
    };
    let slider_focused = sliders.contains(focused_entity);
    for gamepad in gamepads.iter() {
        for button in gamepad.get_just_pressed() {
            let input = match button {
                GamepadButton::DPadUp => MenuInput::Up,
                GamepadButton::DPadDown => MenuInput::Down,
                // Skip left/right for sliders since they're handled in get_pressed loop
                GamepadButton::DPadLeft if !slider_focused => MenuInput::Left,
                GamepadButton::DPadRight if !slider_focused => MenuInput::Right,
                GamepadButton::North => MenuInput::Delete,
                GamepadButton::South => MenuInput::Select,
                GamepadButton::East => MenuInput::Back,
                _ => continue,
            };
            commands.trigger(MenuInputEvent {
                entity: focused_entity,
                input,
            });
        }
        if slider_focused {
            for button in gamepad.get_pressed() {
                let input = match button {
                    GamepadButton::DPadLeft => MenuInput::Left,
                    GamepadButton::DPadRight => MenuInput::Right,
                    _ => continue,
                };
                commands.trigger(MenuInputEvent {
                    entity: focused_entity,
                    input,
                });
            }
        }
    }
}
