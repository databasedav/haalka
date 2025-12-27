//! - Main menu with sub menus for audio and graphics.
//! - Simple buttons for option selection.
//! - Slider for volume.
//! - Dropdown for graphics quality (low/medium/high).
//! - Navigation possible with mouse, keyboard and controller.
//!   - Mouse: Separate styles for hover and press.
//!   - Keyboard/Controller: Separate styles for currently focused element.

mod utils;
use haalka::impl_haalka_methods;
use utils::*;

use bevy::{prelude::*, ui::Pressed};
use haalka::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

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
        .init_resource::<SubMenuHoveredIndex>()
        .insert_resource(FocusedEntity(Entity::PLACEHOLDER))
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

#[derive(Clone, Copy, PartialEq, Display, EnumIter)]
enum SubMenu {
    Audio,
    Graphics,
}

#[derive(Resource, Clone, Copy, PartialEq, Default)]
struct ShowSubMenu(Option<SubMenu>);

#[derive(Resource, Clone, Copy, PartialEq, Default)]
struct SubMenuSelected(Option<SubMenu>);

/// Component to mark a button as selected (for external control)
#[derive(Component, Clone, Default)]
struct Selected;

// core widget, pretty much every other widget uses the `Button`
#[derive(Default)]
struct Button {
    el: El<Node>,
    #[allow(dead_code)]
    lazy_entity: LazyEntity,
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

        let pressed_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .map(|In(entity), presseds: Query<&Pressed>| presseds.contains(entity))
            .dedupe();

        let hovered_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Hovered>()
            .dedupe();

        let selected_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Selected>()
            .dedupe();

        let selected_hovered_signal = selected_signal
            .clone()
            .combine(pressed_signal)
            .map_in(|(selected, pressed)| selected || pressed)
            .combine(hovered_signal.clone())
            .dedupe();

        Self {
            el: {
                El::<Node>::new()
                    .lazy_entity(lazy_entity.clone())
                    .insert(Pickable::default())
                    .with_node(|mut node| {
                        node.height = Val::Px(DEFAULT_BUTTON_HEIGHT);
                        node.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                    })
                    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                    .align_content(Align::center())
                    .border_color_signal(
                        selected_hovered_signal
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
                        selected_hovered_signal
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
            lazy_entity,
        }
    }

    fn body(mut self, body: impl Element) -> Self {
        self.el = self.el.child(body);
        self
    }

    fn selected_signal(mut self, selected_signal: impl Signal<Item = bool> + Clone + Send + 'static) -> Self {
        self.el = self
            .el
            .with_builder(|builder| builder.component_signal::<Selected, _>(selected_signal.map_true_in(|| Selected)));
        self
    }

    fn hovered_signal(mut self, hovered_signal: impl Signal<Item = bool> + Clone + Send + 'static) -> Self {
        self.el = self
            .el
            .with_builder(|builder| builder.component_signal::<Hovered, _>(hovered_signal.map_true_in(|| Hovered)));
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

/// Component marker for entities that are being controlled (focused)
#[derive(Component, Clone, Default)]
struct Controlling;

#[allow(dead_code)]
trait Controllable: ElementWrapper + BuilderPassThrough
where
    Self: Sized + 'static,
{
    fn lazy_entity(&self) -> LazyEntity;

    fn controlling_signal(self, controlling_signal: impl Signal<Item = bool> + Clone + Send + 'static) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<Controlling, _>(controlling_signal.map_true_in(|| Controlling))
        })
    }
}

#[derive(Component, Clone, PartialEq)]
struct MenuInputDisabled;

/// Component to store checkbox checked state
#[derive(Component, Clone, Default)]
struct Checked;

/// Component to store the checkbox's LazyEntity reference
#[derive(Component, Clone)]
struct CheckboxEntity(LazyEntity);

struct Checkbox {
    el: Button,
    #[allow(dead_code)]
    lazy_entity: LazyEntity,
}

impl Checkbox {
    fn new(initially_checked: bool) -> Self {
        let lazy_entity = LazyEntity::new();
        let controlling_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Controlling>()
            .dedupe();
        Self {
            el: {
                lil_baby_button()
                    .apply(|element| focus_on_signal(element, controlling_signal.clone()))
                    .with_builder(clone!((lazy_entity) move |builder| {
                        builder
                            .lazy_entity(lazy_entity.clone())
                            .insert(CheckboxEntity(lazy_entity.clone()))
                            .component_signal::<MenuInputDisabled, _>(
                                controlling_signal.clone().not().map_true_in(|| MenuInputDisabled),
                            )
                    }))
                    .observe(
                        move |event: On<MenuInputEvent>,
                              disabled: Query<&MenuInputDisabled>,
                              checkeds: Query<&Checked>,
                              checkbox_entities: Query<&CheckboxEntity>,
                              mut commands: Commands| {
                            if disabled.contains(event.entity) {
                                return;
                            }
                            let Ok(CheckboxEntity(lazy_entity)) = checkbox_entities.get(event.entity) else {
                                return;
                            };
                            let entity = **lazy_entity;
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
                        },
                    )
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
                        SignalBuilder::from_lazy_entity(lazy_entity.clone())
                            .has_component::<Checked>()
                            .dedupe(),
                    )
            }
            .also(|el| {
                if initially_checked {
                    el.el = el.el.clone().insert(Checked);
                }
            }),
            lazy_entity,
        }
    }
}

impl ElementWrapper for Checkbox {
    type EL = Button;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl BuilderPassThrough for Checkbox {}

impl Controllable for Checkbox {
    fn lazy_entity(&self) -> LazyEntity {
        self.lazy_entity.clone()
    }
}

#[derive(Clone, Copy, EnumIter, PartialEq, Display)]
enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

/// Component to store radio group selection
#[derive(Component, Clone, Default)]
struct RadioSelection(Option<usize>);

/// Component to store the radio group's options
#[derive(Component, Clone)]
struct RadioOptions(Vec<String>);

struct RadioGroup {
    el: Row<Node>,
    #[allow(dead_code)]
    lazy_entity: LazyEntity,
}

impl RadioGroup {
    fn new(options: Vec<String>, initial_selection: Option<usize>) -> Self {
        let lazy_entity = LazyEntity::new();
        let controlling_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Controlling>()
            .dedupe();
        let opts = options.clone();
        let _opts_len = options.len();
        Self {
            el: {
                Row::<Node>::new()
                    .apply(|element| focus_on_signal(element, controlling_signal.clone()))
                    .with_builder(clone!((lazy_entity, options) move |builder| {
                        builder
                            .lazy_entity(lazy_entity.clone())
                            .insert(RadioSelection(initial_selection))
                            .insert(RadioOptions(options))
                            .component_signal::<MenuInputDisabled, _>(
                                controlling_signal.clone().not().map_true_in(|| MenuInputDisabled),
                            )
                    }))
                    .observe(clone!((lazy_entity) move |event: On<MenuInputEvent>,
                          disabled: Query<&MenuInputDisabled>,
                          mut selections: Query<(&mut RadioSelection, &RadioOptions)>| {
                        if disabled.contains(event.entity) {
                            return;
                        }
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
                                    SignalBuilder::always(option.clone()),
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
                                    SignalBuilder::from_component_lazy::<RadioSelection>(lazy_entity.clone())
                                        .map_in(move |RadioSelection(sel)| sel == Some(i))
                                        .dedupe(),
                                )
                            })),
                    )
            },
            lazy_entity,
        }
    }
}

impl ElementWrapper for RadioGroup {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl BuilderPassThrough for RadioGroup {}

impl Controllable for RadioGroup {
    fn lazy_entity(&self) -> LazyEntity {
        self.lazy_entity.clone()
    }
}

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
#[derive(Component, Clone)]
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
    #[allow(dead_code)]
    lazy_entity: LazyEntity,
}

impl IterableOptions {
    fn new(options: Vec<String>, initial_selection: String) -> Self {
        let lazy_entity = LazyEntity::new();
        let controlling_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Controlling>()
            .dedupe();
        let opts = options.clone();
        Self {
            el: {
                Row::<Node>::new()
                    .apply(|element| focus_on_signal(element, controlling_signal.clone()))
                    .with_builder(clone!((lazy_entity, options, initial_selection) move |builder| {
                        builder
                            .lazy_entity(lazy_entity.clone())
                            .insert(IterableSelection(initial_selection))
                            .insert(IterableOptionsList(options))
                            .component_signal::<MenuInputDisabled, _>(
                                controlling_signal.clone().not().map_true_in(|| MenuInputDisabled),
                            )
                    }))
                    .observe(
                        clone!((lazy_entity) move |event: On<MenuInputEvent>,
                              disabled: Query<&MenuInputDisabled>,
                              mut iterables: Query<(&mut IterableSelection, &IterableOptionsList)>,
                              mut commands: Commands| {
                            if disabled.contains(event.entity) {
                                return;
                            }
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
                                SignalBuilder::from_lazy_entity(lazy_entity.clone())
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
                                SignalBuilder::from_component_lazy::<IterableSelection>(lazy_entity.clone())
                                    .map_in(|IterableSelection(s)| s)
                                    .dedupe()
                                    .map_in(Text)
                                    .map_in(Some),
                            ),
                    )
                    .item({
                        lil_baby_button()
                            .selected_signal(
                                SignalBuilder::from_lazy_entity(lazy_entity.clone())
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
            },
            lazy_entity,
        }
    }
}

impl ElementWrapper for IterableOptions {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl BuilderPassThrough for IterableOptions {}

impl Controllable for IterableOptions {
    fn lazy_entity(&self) -> LazyEntity {
        self.lazy_entity.clone()
    }
}

/// Component to store slider value
#[derive(Component, Clone)]
struct SliderValue(f32);

/// Component to store slider left position
#[derive(Component, Clone)]
struct SliderLeft(f32);

/// Component marker for slider being dragged
#[derive(Component, Clone, Default)]
struct SliderDragging;

struct Slider {
    el: Row<Node>,
    #[allow(dead_code)]
    lazy_entity: LazyEntity,
}

impl Slider {
    fn new(initial_value: f32) -> Self {
        let lazy_entity = LazyEntity::new();
        let controlling_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Controlling>()
            .dedupe();
        let slider_width = 400.;
        let slider_padding = 5.;
        let max = slider_width - slider_padding - LIL_BABY_BUTTON_SIZE - BASE_BORDER_WIDTH;
        let initial_left = initial_value / 100. * max;

        Self {
            el: {
                Row::<Node>::new()
                    .insert(SliderTag)
                    .apply(|element| focus_on_signal(element, controlling_signal.clone()))
                    .with_builder(clone!((lazy_entity) move |builder| {
                        builder
                            .lazy_entity(lazy_entity.clone())
                            .insert(SliderValue(initial_value))
                            .insert(SliderLeft(initial_left))
                            .component_signal::<MenuInputDisabled, _>(
                                controlling_signal.clone().not().map_true_in(|| MenuInputDisabled),
                            )
                    }))
                    .observe(
                        clone!((lazy_entity) move |event: On<MenuInputEvent>,
                              disabled: Query<&MenuInputDisabled>,
                              mut sliders: Query<(&mut SliderLeft, &mut SliderValue)>| {
                            if disabled.contains(event.entity) {
                                return;
                            }
                            let Ok((mut left, mut value)) = sliders.get_mut(*lazy_entity) else {
                                return;
                            };
                            match event.input {
                                MenuInput::Left | MenuInput::Right => {
                                    let dir = if matches!(event.input, MenuInput::Left) { -1. } else { 1. };
                                    left.0 = (left.0 + dir * max * 0.001).max(0.).min(max);
                                    value.0 = left.0 / max * 100.;
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
                                SignalBuilder::from_component_lazy::<SliderValue>(lazy_entity.clone())
                                    .map_in(|SliderValue(v)| v)
                                    .dedupe()
                                    .map_in(|value| Text(format!("{value:.1}")))
                                    .map_in(Some),
                            ),
                    )
                    .item(
                        Stack::<Node>::new()
                            .with_node(move |mut node| {
                                node.width = Val::Px(slider_width);
                                node.height = Val::Px(5.);
                                node.padding = UiRect::horizontal(Val::Px(slider_padding));
                            })
                            .background_color(BackgroundColor(Color::BLACK))
                            .layer({
                                let knob_entity = LazyEntity::new();
                                lil_baby_button()
                                    .selected_signal(
                                        SignalBuilder::from_lazy_entity(knob_entity.clone())
                                            .has_component::<SliderDragging>()
                                            .dedupe(),
                                    )
                                    .el // we need lower level access now
                                    .lazy_entity(knob_entity.clone())
                                    .insert(Pickable::default())
                                    .on_signal_with_node(
                                        SignalBuilder::from_component_lazy::<SliderLeft>(lazy_entity.clone())
                                            .map_in(|SliderLeft(l)| l)
                                            .dedupe(),
                                        |mut node, left| node.left = Val::Px(left),
                                    )
                                    .align(Align::new().center_y())
                                    .observe(clone!((knob_entity) move |_: On<Pointer<DragStart>>, mut commands: Commands| {
                                        commands.entity(*knob_entity).insert(SliderDragging);
                                    }))
                                    .observe(clone!((knob_entity) move |_: On<Pointer<DragEnd>>, mut commands: Commands| {
                                        commands.entity(*knob_entity).remove::<SliderDragging>();
                                    }))
                                    .observe(clone!((lazy_entity) move |drag: On<Pointer<Drag>>, mut sliders: Query<(&mut SliderLeft, &mut SliderValue)>| {
                                        let Ok((mut left, mut value)) = sliders.get_mut(*lazy_entity) else {
                                            return;
                                        };
                                        left.0 = (left.0 + drag.delta.x).max(0.).min(max);
                                        value.0 = left.0 / max * 100.;
                                    }))
                            }),
                    )
            },
            lazy_entity,
        }
    }
}

impl ElementWrapper for Slider {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl BuilderPassThrough for Slider {}

impl Controllable for Slider {
    fn lazy_entity(&self) -> LazyEntity {
        self.lazy_entity.clone()
    }
}

fn options(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("option {i}")).collect()
}

/// Component marker for menu items that are hovered
#[derive(Component, Clone, Default)]
struct MenuItemHovered;

fn menu_item(label: &str, body: impl Element, item_entity: LazyEntity) -> Stack<Node> {
    Stack::<Node>::new()
        .lazy_entity(item_entity.clone())
        .insert(Pickable::default())
        .background_color_signal(
            SignalBuilder::from_lazy_entity(item_entity.clone())
                .has_component::<MenuItemHovered>()
                .dedupe()
                .map_bool_in(|| NORMAL_BUTTON.lighter(0.05), || NORMAL_BUTTON)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .on_hovered_change(
            clone!((item_entity) move |In((_, data)): In<(Entity, HoverData)>, mut commands: Commands| {
                if data.hovered {
                    commands.entity(*item_entity).insert(MenuItemHovered);
                } else {
                    commands.entity(*item_entity).remove::<MenuItemHovered>();
                }
            }),
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

/// Component to store dropdown selection (as string for simplicity)
#[derive(Component, Clone, Default)]
struct DropdownSelection(Option<String>);

/// Component to store dropdown options
#[derive(Component, Clone)]
struct DropdownOptions(Vec<String>);

/// Component marker for dropdown being shown
#[derive(Component, Clone, Default)]
struct DropdownShowing;

/// Component to mark which option index is hovered in dropdown
#[derive(Component, Clone, Default)]
struct DropdownHoveredIndex(Option<usize>);

/// Component marker for dropdown being clearable
#[derive(Component, Clone, Default)]
struct DropdownClearable;

struct Dropdown {
    el: El<Node>,
    lazy_entity: LazyEntity,
}

fn focus_on_signal<E: Element>(element: E, signal: impl Signal<Item = bool> + Clone + Send + 'static) -> E {
    element.with_builder(|builder| {
        builder.on_signal(signal.dedupe(), |In((entity, focus)), mut commands: Commands| {
            if focus {
                commands.insert_resource(FocusedEntity(entity));
            }
        })
    })
}

impl Dropdown {
    fn new(options: Vec<String>, initial_selection: Option<String>, clearable: bool) -> Self {
        let lazy_entity = LazyEntity::new();
        let controlling_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<Controlling>()
            .dedupe();
        let show_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
            .has_component::<DropdownShowing>()
            .dedupe();
        let opts = options.clone();
        let _opts_len = options.len();

        let el = El::<Node>::new()
            .apply(|element| focus_on_signal(element, controlling_signal.clone()))
            .with_builder(clone!((lazy_entity, options, initial_selection) move |builder| {
                let mut b = builder
                    .lazy_entity(lazy_entity.clone())
                    .insert(DropdownSelection(initial_selection))
                    .insert(DropdownOptions(options))
                    .insert(DropdownHoveredIndex(None))
                    .component_signal::<MenuInputDisabled, _>(
                        controlling_signal.clone().not().map_true_in(|| MenuInputDisabled),
                    );
                if clearable {
                    b = b.insert(DropdownClearable);
                }
                b
            }))
            .observe(
                clone!((lazy_entity) move |mut event: On<MenuInputEvent>,
                      controlleds: Query<&Controlling>,
                      showings: Query<&DropdownShowing>,
                      clearables: Query<&DropdownClearable>,
                      mut dropdowns: Query<(&mut DropdownSelection, &DropdownOptions, &mut DropdownHoveredIndex)>,
                      mut commands: Commands| {
                    if !controlleds.contains(*lazy_entity) {
                        return;
                    }
                    let Ok((mut selection, options, mut hovered_idx)) = dropdowns.get_mut(*lazy_entity) else {
                        return;
                    };
                    let is_showing = showings.contains(*lazy_entity);
                    let is_clearable = clearables.contains(*lazy_entity);

                    match event.input {
                        MenuInput::Up | MenuInput::Down => {
                            if is_showing {
                                event.propagate(false);
                                let step = if matches!(event.input, MenuInput::Up) { -1isize } else { 1isize };
                                let mut i = hovered_idx.0.unwrap_or(if step < 0 { options.0.len() - 1 } else { 0 }) as isize;
                                if hovered_idx.0.is_some() {
                                    i = (i + step + options.0.len() as isize) % options.0.len() as isize;
                                }
                                // Skip currently selected option
                                if selection.0.as_ref() == Some(&options.0[i as usize]) {
                                    i = (i + step + options.0.len() as isize) % options.0.len() as isize;
                                }
                                hovered_idx.0 = Some(i as usize);
                            }
                        }
                        MenuInput::Select => {
                            if let Some(i) = hovered_idx.0 {
                                selection.0 = Some(options.0[i].clone());
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
                            .layer(
                                El::<Text>::new()
                                    .align(Align::new().left())
                                    .text_font(TextFont::from_font_size(FONT_SIZE))
                                    .text_signal(
                                        SignalBuilder::from_component_lazy::<DropdownSelection>(lazy_entity.clone())
                                            .map_in(|DropdownSelection(s)| s.clone().unwrap_or_default())
                                            .dedupe()
                                            .map_in(Text)
                                            .map_in(Some),
                                    ),
                            )
                            .layer(
                                Row::<Node>::new()
                                    .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING))
                                    .align(Align::new().right())
                                    .item_signal(if clearable {
                                        SignalBuilder::from_component_lazy::<DropdownSelection>(lazy_entity.clone())
                                            .map_in(|DropdownSelection(s)| s.is_some())
                                            .dedupe()
                                            .map_true_in(clone!((lazy_entity) move || {
                                                x_button(clone!((lazy_entity) move |_: In<_>, mut selections: Query<&mut DropdownSelection>| {
                                                    if let Ok(mut sel) = selections.get_mut(*lazy_entity) {
                                                        sel.0 = None;
                                                    }
                                                }))
                                            }))
                                            .left_either()
                                    } else {
                                        SignalBuilder::always(None::<El<Node>>).right_either()
                                    })
                                    .item(
                                        El::<Text>::new()
                                            .text_font(TextFont::from_font_size(FONT_SIZE))
                                            .text(Text::new("v")),
                                    ),
                            ),
                    )
                    .on_click(clone!((lazy_entity) move |_: In<_>, showings: Query<&DropdownShowing>, mut commands: Commands| {
                        if showings.contains(*lazy_entity) {
                            commands.entity(*lazy_entity).remove::<DropdownShowing>();
                        } else {
                            commands.entity(*lazy_entity).insert(DropdownShowing);
                        }
                    })),
            )
            .child_signal(
                show_signal.map_true_in(clone!((lazy_entity, opts) move || {
                    Column::<Node>::new()
                        .with_node(|mut node| {
                            node.width = Val::Percent(100.);
                            node.position_type = PositionType::Absolute;
                            node.top = Val::Percent(100.);
                        })
                        .items(opts.clone().into_iter().enumerate().map(clone!((lazy_entity) move |(i, opt)| {
                            text_button(
                                SignalBuilder::always(opt.clone()),
                                clone!((lazy_entity, opt) move |_: In<_>, mut dropdowns: Query<&mut DropdownSelection>, showings: Query<&DropdownShowing>, mut commands: Commands| {
                                    if let Ok(mut sel) = dropdowns.get_mut(*lazy_entity) {
                                        sel.0 = Some(opt.clone());
                                    }
                                    if showings.contains(*lazy_entity) {
                                        commands.entity(*lazy_entity).remove::<DropdownShowing>();
                                    }
                                }),
                            )
                            .with_node(|mut node| node.width = Val::Percent(100.))
                            .selected_signal(
                                SignalBuilder::from_component_lazy::<DropdownHoveredIndex>(lazy_entity.clone())
                                    .map_in(move |DropdownHoveredIndex(idx)| idx == Some(i))
                                    .dedupe(),
                            )
                        })))
                })),
            );

        Self { el, lazy_entity }
    }
}

impl ElementWrapper for Dropdown {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl BuilderPassThrough for Dropdown {}

impl Controllable for Dropdown {
    fn lazy_entity(&self) -> LazyEntity {
        self.lazy_entity.clone()
    }
}

/// Resource to track which menu items are hovered by index
#[derive(Resource, Clone, Default)]
struct SubMenuHoveredIndex(Option<usize>);

fn sub_menu_child_hover_manager<E: Element + BuilderPassThrough>(element: E, num_items: usize) -> E {
    element.observe(
        move |event: On<MenuInputEvent>, mut hovered_idx: ResMut<SubMenuHoveredIndex>, mut commands: Commands| {
            match event.input {
                MenuInput::Up | MenuInput::Down => {
                    let step = if matches!(event.input, MenuInput::Up) {
                        -1isize
                    } else {
                        1isize
                    };
                    let i = match hovered_idx.0 {
                        Some(cur) => ((cur as isize + step + num_items as isize) % num_items as isize) as usize,
                        None => {
                            if step < 0 {
                                num_items - 1
                            } else {
                                0
                            }
                        }
                    };
                    hovered_idx.0 = Some(i);
                }
                MenuInput::Back => {
                    if hovered_idx.0.is_some() {
                        hovered_idx.0 = None;
                    } else {
                        commands.insert_resource(ShowSubMenu(None));
                    }
                }
                _ => (),
            }
        },
    )
}

fn make_controlling_menu_item(label: &str, el: impl Controllable + Element, item_index: usize) -> Stack<Node> {
    let item_entity = LazyEntity::new();
    let hovered_signal = SignalBuilder::from_resource::<SubMenuHoveredIndex>()
        .map_in(move |SubMenuHoveredIndex(idx)| idx == Some(item_index))
        .dedupe();
    menu_item(
        label,
        el.controlling_signal(hovered_signal.clone()),
        item_entity.clone(),
    )
    .with_builder(|builder| {
        builder.component_signal::<MenuItemHovered, _>(hovered_signal.map_true_in(|| MenuItemHovered))
    })
}

fn audio_menu() -> Column<Node> {
    let items = [
        make_controlling_menu_item("dropdown", Dropdown::new(options(4), None, true), 0),
        make_controlling_menu_item("radio group", RadioGroup::new(options(3), None), 1),
        make_controlling_menu_item("checkbox", Checkbox::new(false), 2),
        make_controlling_menu_item(
            "iterable options",
            IterableOptions::new(options(4), "option 1".to_string()),
            3,
        ),
        make_controlling_menu_item("master volume", Slider::new(100.), 4),
        make_controlling_menu_item("effect volume", Slider::new(50.), 5),
        make_controlling_menu_item("music volume", Slider::new(50.), 6),
        make_controlling_menu_item("voice volume", Slider::new(50.), 7),
    ];
    let l = items.len();
    let no_child_hovered = SignalBuilder::from_resource::<SubMenuHoveredIndex>()
        .map_in(|SubMenuHoveredIndex(idx)| idx.is_none())
        .dedupe();
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "audio menu")
        .apply(|element| focus_on_signal(element, no_child_hovered))
        .apply(|element| sub_menu_child_hover_manager(element, l))
        .items(
            items
                .into_iter()
                .enumerate()
                .map(move |(i, item)| item.z_index(ZIndex((l - i) as i32))),
        )
}

fn graphics_menu() -> Column<Node> {
    let l = 4usize;

    // Create dropdowns with their lazy entities exposed for signal sync
    let preset_dropdown = Dropdown::new(
        Quality::iter().map(|q| q.to_string()).collect::<Vec<_>>(),
        Some(Quality::Medium.to_string()),
        true,
    );
    let preset_entity = preset_dropdown.lazy_entity.clone();

    let texture_dropdown = Dropdown::new(
        Quality::iter().map(|q| q.to_string()).collect::<Vec<_>>(),
        Some(Quality::Medium.to_string()),
        false,
    );
    let texture_entity = texture_dropdown.lazy_entity.clone();

    let shadow_dropdown = Dropdown::new(
        Quality::iter().map(|q| q.to_string()).collect::<Vec<_>>(),
        Some(Quality::Medium.to_string()),
        false,
    );
    let shadow_entity = shadow_dropdown.lazy_entity.clone();

    let bloom_dropdown = Dropdown::new(
        Quality::iter().map(|q| q.to_string()).collect::<Vec<_>>(),
        Some(Quality::Medium.to_string()),
        false,
    );
    let bloom_entity = bloom_dropdown.lazy_entity.clone();

    // Signal: When preset changes, propagate to all individual qualities
    #[allow(clippy::type_complexity)]
    let preset_broadcaster = SignalBuilder::from_component_lazy::<DropdownSelection>(preset_entity.clone())
        .map_in(|DropdownSelection(sel)| sel)
        .dedupe()
        .map(clone!((texture_entity, shadow_entity, bloom_entity) move |In(preset_quality): In<Option<String>>, mut selections: Query<&mut DropdownSelection>| {
            if let Some(ref quality) = preset_quality {
                if let Ok(mut texture) = selections.get_mut(*texture_entity)
                    && texture.0.as_ref() != Some(quality)
                {
                    texture.0 = Some(quality.clone());
                }
                if let Ok(mut shadow) = selections.get_mut(*shadow_entity)
                    && shadow.0.as_ref() != Some(quality)
                {
                    shadow.0 = Some(quality.clone());
                }
                if let Ok(mut bloom) = selections.get_mut(*bloom_entity)
                    && bloom.0.as_ref() != Some(quality)
                {
                    bloom.0 = Some(quality.clone());
                }
            }
        }))
        .hold();

    // Signal: When individual qualities change, update preset if they all match
    #[allow(clippy::type_complexity)]
    let preset_controller = SignalBuilder::from_component_lazy::<DropdownSelection>(texture_entity.clone())
        .map_in(|DropdownSelection(sel)| sel)
        .combine(
            SignalBuilder::from_component_lazy::<DropdownSelection>(shadow_entity.clone())
                .map_in(|DropdownSelection(sel)| sel)
        )
        .combine(
            SignalBuilder::from_component_lazy::<DropdownSelection>(bloom_entity.clone())
                .map_in(|DropdownSelection(sel)| sel)
        )
        .dedupe()
        .map(clone!((preset_entity) move |In(((texture, shadow), bloom)): In<((Option<String>, Option<String>), Option<String>)>, mut selections: Query<&mut DropdownSelection>| {
            let Ok(mut preset) = selections.get_mut(*preset_entity) else { return };

            // Check if all three individual qualities match
            if texture.is_some() && texture == shadow && shadow == bloom {
                // All match - set preset to this value if different
                if preset.0 != texture {
                    preset.0 = texture;
                }
            } else {
                // They don't all match - clear preset if it's set
                if preset.0.is_some() {
                    preset.0 = None;
                }
            }
        }))
        .hold();

    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "graphics menu")
        .apply(|element| {
            focus_on_signal(
                element,
                SignalBuilder::from_resource::<SubMenuHoveredIndex>()
                    .map_in(|SubMenuHoveredIndex(index)| index.is_none())
                    .dedupe(),
            )
        })
        .apply(|element| sub_menu_child_hover_manager(element, l))
        .item(make_controlling_menu_item("preset quality", preset_dropdown, 0).z_index(ZIndex(4)))
        .item(make_controlling_menu_item("texture quality", texture_dropdown, 1).z_index(ZIndex(3)))
        .item(make_controlling_menu_item("shadow quality", shadow_dropdown, 2).z_index(ZIndex(2)))
        .item(make_controlling_menu_item("bloom quality", bloom_dropdown, 3).z_index(ZIndex(1)))
        .item(
            // solely here to dehover dropdown menu items  // TODO: this can also be solved by
            // allowing setting Over/Out order at runtime or implementing .on_hovered_outside, i
            // should do both of these
            El::<Node>::new()
                .with_node(move |mut node| {
                    node.height = Val::Px(SUB_MENU_HEIGHT - (l + 1) as f32 * MENU_ITEM_HEIGHT - BASE_PADDING * 2.)
                })
                .on_hovered_change(
                    |In((_, data)): In<(Entity, HoverData)>, mut hovered_index: ResMut<SubMenuHoveredIndex>| {
                        if data.hovered {
                            hovered_index.0 = None;
                        }
                    },
                ),
        )
        .with_builder(|b| b.hold_signals([preset_broadcaster, preset_controller]))
}

fn x_button<Marker>(
    on_click: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
) -> El<Node> {
    let lazy = LazyEntity::new();
    El::<Node>::new()
        .lazy_entity(lazy.clone())
        .background_color(BackgroundColor(Color::NONE))
        .insert(Pickable::default())
        .on_click(on_click)
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new("x"))
                .text_color_signal(
                    SignalBuilder::from_lazy_entity(lazy)
                        .has_component::<Hovered>()
                        .map_bool_in(|| bevy::color::palettes::basic::RED.into(), || TEXT_COLOR)
                        .map_in(TextColor)
                        .map_in(Some),
                ),
        )
}

fn menu() -> impl Element {
    Stack::<Node>::new()
        .layer(
            menu_base(MAIN_MENU_SIDES, MAIN_MENU_SIDES, "main menu")
                .apply(|element| {
                    focus_on_signal(
                        element,
                        SignalBuilder::from_resource::<ShowSubMenu>()
                            .map_in(|ShowSubMenu(option)| option.is_none())
                            .dedupe(),
                    )
                })
                .with_builder(|b| {
                    b.component_signal::<MenuInputDisabled, _>(
                        SignalBuilder::from_resource::<ShowSubMenu>()
                            .map_in(|ShowSubMenu(option)| option.is_some())
                            .dedupe()
                            .map_true_in(|| MenuInputDisabled),
                    )
                })
                .observe(
                    move |event: On<MenuInputEvent>,
                          disabled: Query<&MenuInputDisabled>,
                          sub_menu_selected: Res<SubMenuSelected>,
                          mut commands: Commands| {
                        if disabled.contains(event.entity) {
                            return;
                        }
                        match event.input {
                            MenuInput::Up | MenuInput::Down => {
                                if let Some(cur_sub_menu) = sub_menu_selected.0 {
                                    if let Some(i) = SubMenu::iter().position(|sub_menu| cur_sub_menu == sub_menu) {
                                        let sub_menus = SubMenu::iter().collect::<Vec<_>>();
                                        commands.insert_resource(SubMenuSelected(
                                            if matches!(event.input, MenuInput::Down) {
                                                sub_menus.iter().rev().cycle().nth(sub_menus.len() - i).copied()
                                            } else {
                                                sub_menus.iter().cycle().nth(i + 1).copied()
                                            },
                                        ));
                                    }
                                } else {
                                    commands.insert_resource(SubMenuSelected(Some(
                                        if matches!(event.input, MenuInput::Up) {
                                            SubMenu::iter().next_back().unwrap()
                                        } else {
                                            SubMenu::iter().next().unwrap()
                                        },
                                    )));
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
                                SignalBuilder::from_resource::<SubMenuSelected>()
                                    .map_in(move |SubMenuSelected(selected_option)| selected_option == Some(sub_menu))
                                    .dedupe(),
                            )
                        })),
                ),
        )
        .layer_signal(
            SignalBuilder::from_resource::<ShowSubMenu>()
                .map_in(|ShowSubMenu(option)| option)
                .dedupe()
                .map_some_in(move |sub_menu| {
                    let menu: Column<Node> = match sub_menu {
                        SubMenu::Audio => audio_menu(),
                        SubMenu::Graphics => graphics_menu(),
                    };
                    Stack::<Node>::new()
                        .with_node(|mut node| {
                            node.width = Val::Px(SUB_MENU_WIDTH);
                            node.height = Val::Px(SUB_MENU_HEIGHT);
                            // TODO: without absolute there's some weird bouncing when switching between
                            // menus, perhaps due to the layout system having to figure stuff out ?
                            node.position_type = PositionType::Absolute;
                        })
                        .align(Align::center())
                        .layer(menu.align(Align::center()))
                        .layer(
                            x_button(|_: In<_>, mut commands: Commands| {
                                commands.insert_resource(ShowSubMenu(None));
                            })
                            .align(Align::new().top().right())
                            .with_node(|mut node| {
                                node.padding.right = Val::Px(BASE_PADDING);
                                node.padding.top = Val::Px(BASE_PADDING / 2.);
                            }),
                        )
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
    focused_entity: Res<FocusedEntity>,
    keys: Res<ButtonInput<KeyCode>>,
    mut menu_input_rate_limiter: ResMut<MenuInputRateLimiter>,
    mut slider_rate_limiter: ResMut<SliderRateLimiter>,
    time: Res<Time>,
    mut commands: Commands,
) {
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
            focused_entity.0,
            &mut menu_input_rate_limiter.0,
            &time,
            &mut commands,
        );
        if handled {
            return;
        }
    }
    let slider_focused = sliders.get(focused_entity.0).is_ok();
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
        rate_limited_menu_input(
            pressed_type,
            input,
            focused_entity.0,
            rate_limiter,
            &time,
            &mut commands,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn gamepad_menu_input_events(
    sliders: Query<Entity, With<SliderTag>>,
    focused_entity: Res<FocusedEntity>,
    gamepads: Query<&Gamepad>,
    mut menu_input_rate_limiter: ResMut<MenuInputRateLimiter>,
    mut slider_rate_limiter: ResMut<SliderRateLimiter>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let slider_focused = sliders.get(focused_entity.0).is_ok();
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
            rate_limited_menu_input(
                pressed_type,
                input,
                focused_entity.0,
                rate_limiter,
                &time,
                &mut commands,
            );
        }
    }
}

#[derive(Resource)]
struct FocusedEntity(Entity);

const MENU_INPUT_RATE_LIMIT: f32 = 0.15;
const SLIDER_RATE_LIMIT: f32 = 0.001;

fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(menu())
}
