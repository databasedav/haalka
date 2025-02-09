//! - Main menu with sub menus for audio and graphics.
//! - Simple buttons for option selection.
//! - Slider for volume.
//! - Dropdown for graphics quality (low/medium/high).
//! - Navigation possible with mouse, keyboard and controller.
//!   - Mouse: Separate styles for hover and press.
//!   - Keyboard/Controller: Separate styles for currently focused element.

mod utils;
use utils::*;

use std::{convert::identity, fmt::Display, time::Duration};

use bevy::prelude::*;
use haalka::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_event::<MenuInput>()
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
        .insert_resource(AUDIO_SETTINGS.clone())
        .insert_resource(GRAPHICS_SETTINGS.clone())
        .insert_resource(MISC_DEMO_SETTINGS.clone())
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

// core widget, pretty much every other widget uses the `Button`
#[derive(Default)]
struct Button {
    el: El<Node>,
    selected: Mutable<bool>,
    hovered: Mutable<bool>,
}

// implementing `ElementWrapper` allows the struct to be passed directly to .child methods
impl ElementWrapper for Button {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Sizeable for Button {}
impl GlobalEventAware for Button {}
impl PointerEventAware for Button {}

impl Button {
    fn new() -> Self {
        let (selected, selected_signal) = Mutable::new_and_signal(false);
        let (pressed, pressed_signal) = Mutable::new_and_signal(false);
        let (hovered, hovered_signal) = Mutable::new_and_signal(false);
        let selected_hovered_broadcaster = map_ref!(selected_signal, pressed_signal, hovered_signal => (*selected_signal || *pressed_signal, *hovered_signal)).broadcast();
        let border_color_signal = {
            selected_hovered_broadcaster
                .signal()
                .map(|(selected, hovered)| {
                    if selected {
                        bevy::color::palettes::basic::RED.into()
                    } else if hovered {
                        Color::WHITE
                    } else {
                        Color::BLACK
                    }
                })
                .map(Into::into)
        };
        let background_color_signal = {
            selected_hovered_broadcaster
                .signal()
                .map(|(selected, hovered)| {
                    if selected {
                        CLICKED_BUTTON
                    } else if hovered {
                        HOVERED_BUTTON
                    } else {
                        NORMAL_BUTTON
                    }
                })
                .map(Into::into)
        };
        Self {
            el: {
                El::<Node>::new()
                    .height(Val::Px(DEFAULT_BUTTON_HEIGHT))
                    .with_node(|mut node| {
                        node.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                    })
                    .pressed_sync(pressed)
                    .align_content(Align::center())
                    .hovered_sync(hovered.clone())
                    .border_color_signal(border_color_signal)
                    .background_color_signal(background_color_signal)
            },
            selected,
            hovered,
        }
    }

    fn body(mut self, body: impl Element) -> Self {
        self.el = self.el.child(body);
        self
    }

    fn selected_signal(mut self, selected_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        // syncing mutables like this is a helpful pattern for externally controlling reactive state that
        // has default widget-internal behavior; for example, all buttons are selected on press, but
        // what if we want the selectedness to persist? simply add another mutable that gets flipped
        // on click and then pass a signal of that to this method, which is exactly how the
        // `Checkbox` widget is implemented
        let syncer = spawn(sync(selected_signal, self.selected.clone()));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }

    fn hovered_signal(mut self, hovered_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let syncer = spawn(sync(hovered_signal, self.hovered.clone()));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }
}

fn text_button(
    text_signal: impl Signal<Item = String> + Send + 'static,
    on_click: impl FnMut() + Send + Sync + 'static,
) -> Button {
    Button::new()
        .width(Val::Px(200.))
        .body(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text_signal(text_signal.map(Text)),
        )
        .on_click(on_click)
}

fn sub_menu_button(sub_menu: SubMenu) -> Button {
    text_button(always(sub_menu.to_string()), move || {
        SHOW_SUB_MENU.set_neq(Some(sub_menu))
    })
}

fn menu_base(width: f32, height: f32, title: &str) -> Column<Node> {
    Column::<Node>::new()
        .width(Val::Px(width))
        .height(Val::Px(height))
        .with_node(|mut node| node.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH)))
        .border_color(BorderColor(Color::BLACK))
        .background_color(BackgroundColor(NORMAL_BUTTON))
        .item(
            El::<Node>::new()
                .height(Val::Px(MENU_ITEM_HEIGHT))
                .with_node(|mut node| {
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

// global ui state comes in super handy sometimes ...
// here, we use a global to keep track of any dropdowns that are dropped down, passing it to
// `only_one_up_flipper` to ensure only one is dropped down at a time; a mutable for this can be
// managed more locally, but adds significant unwieldiness
static DROPDOWN_SHOWING_OPTION: Lazy<Mutable<Option<Mutable<bool>>>> = Lazy::new(default);

fn lil_baby_button() -> Button {
    Button::new()
        .width(Val::Px(LIL_BABY_BUTTON_SIZE))
        .height(Val::Px(LIL_BABY_BUTTON_SIZE))
}

trait Controllable: ElementWrapper
where
    Self: Sized + 'static,
{
    fn controlling(&self) -> &Mutable<bool>;

    fn controlling_signal(mut self, controlling_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let syncer = spawn(sync(controlling_signal, self.controlling().clone()));
        self = self.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }
}

struct Checkbox {
    el: Button,
    controlling: Mutable<bool>,
}

impl Checkbox {
    fn new(checked: Mutable<bool>) -> Self {
        let (controlling, controlling_signal) = Mutable::new_and_signal(false);
        Self {
            el: {
                lil_baby_button()
                    .apply(|element| focus_on_signal(element, controlling.signal()))
                    .update_raw_el(clone!((checked) move |raw_el| {
                        raw_el.on_event_disableable_signal::<MenuInput>(
                            move |event| {
                                match event {
                                    MenuInput::Select => {
                                        checked.set_neq(!checked.get());
                                    },
                                    MenuInput::Delete => {
                                        checked.set(false);
                                    },
                                    _ => ()
                                }
                            },
                            signal::not(controlling_signal),
                        )
                    }))
                    .on_click(clone!((checked) move || flip(&checked)))
                    .selected_signal(checked.signal())
            },
            controlling,
        }
    }
}

impl ElementWrapper for Checkbox {
    type EL = Button;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for Checkbox {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
    }
}

#[derive(Clone, Copy, EnumIter, PartialEq, Display)]
enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

struct RadioGroup {
    el: Row<Node>,
    controlling: Mutable<bool>,
}

impl RadioGroup {
    fn new<T: Clone + PartialEq + Display + Send + Sync + 'static>(
        options: MutableVec<T>,
        selected: Mutable<Option<usize>>,
    ) -> Self {
        let (controlling, controlling_signal) = Mutable::new_and_signal(false);
        Self {
            el: {
                Row::<Node>::new()
                .apply(|element| focus_on_signal(element, controlling.signal()))
                .update_raw_el(|raw_el| {
                    raw_el.on_event_disableable_signal::<MenuInput>(
                        clone!((options, selected) move |event| {
                            match event {
                                MenuInput::Left | MenuInput::Right => {
                                    let selected_option = selected.lock_ref().as_ref().copied();
                                    let (mut i, step) = {
                                        if matches!(event, MenuInput::Left) {
                                            (selected_option.unwrap_or(options.lock_ref().len() - 1) as isize, -1)
                                        } else {
                                            (selected_option.unwrap_or(0) as isize, 1)
                                        }
                                    };
                                    if selected_option.is_some() {
                                        i = (i + step + options.lock_ref().len() as isize) % options.lock_ref().len() as isize;
                                    }
                                    selected.set(Some(i as usize));
                                },
                                MenuInput::Delete => {
                                    selected.take();
                                },
                                _ => ()
                            }
                        }),
                        signal::not(controlling_signal)
                    )
                })
                .items_signal_vec(
                    options.signal_vec_cloned().enumerate()
                    .map(clone!((selected) move |(i_option_mutable, option)| {
                        text_button(
                            always(option.to_string()),
                            clone!((selected, i_option_mutable) move || {
                                if selected.get() == i_option_mutable.get() {
                                    selected.set(None);
                                } else {
                                    selected.set(i_option_mutable.get());
                                }
                            })
                        )
                        // the `Checkbox` just used a flippable `Mutable<bool>` to persist the selectedness, and we could
                        // have done the same here, e.g. a separate `clicked: Mutable<bool>` for every text button, but then to
                        // get exclusivity we would have iterate over the other `clicked` mutables and flip them; again, this
                        // is a totally valid option, but it's more convenient in this case to centrally track selectedness
                        // with a `Mutable<Option<usize>>` so we get exclusivity for free; also notice that the index from the
                        // `.enumerate` is a mutable, this is because the options vec is also reactive, so the indicies of items
                        // can change, so this solution isn't actually correct for dynamic options, but it's fine for this example
                        .selected_signal(signal_eq(selected.signal_cloned(), i_option_mutable.signal()))
                    }))
                )
            },
            controlling,
        }
    }
}

impl ElementWrapper for RadioGroup {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for RadioGroup {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
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

struct IterableOptions {
    el: Row<Node>,
    controlling: Mutable<bool>,
}

const FLASH_MS: f32 = 50.; // TODO: address background/border color desyncing

impl IterableOptions {
    fn new<T: Clone + PartialEq + Display + Send + Sync + 'static>(
        options: MutableVec<T>,
        selected: Mutable<T>,
    ) -> Self {
        let (controlling, controlling_signal) = Mutable::new_and_signal(false);
        let left_pressed = Mutable::new(false);
        let right_pressed = Mutable::new(false);
        Self {
            el: {
                Row::<Node>::new()
                .apply(|element| focus_on_signal(element, controlling.signal()))
                .update_raw_el(|raw_el| {
                    // TODO: only allowing one flasher like this doesn't prevent desyncing either ...
                    let left_flasher = Mutable::new(None);
                    let right_flasher = Mutable::new(None);
                    raw_el.on_event_disableable_signal::<MenuInput>(
                        clone!((options, selected, left_pressed, right_pressed) move |event| {
                            match event {
                                MenuInput::Left | MenuInput::Right => {
                                    let i_option = options.lock_ref().iter().position(|option| option == &*selected.lock_ref()).map(|i| i as isize);
                                    if let Some(mut i) = i_option {
                                        let step = {
                                            (if matches!(event, MenuInput::Left) {
                                                left_pressed.set(true);
                                                left_flasher.set(Some(spawn(clone!((left_pressed) async move {
                                                    sleep(Duration::from_millis(FLASH_MS as u64)).await;
                                                    left_pressed.signal().wait_for(true).await;  // TODO: this doesn't prevent desyncing, could be lower level issue ...
                                                    left_pressed.set(false);
                                                }))));
                                                -1
                                            } else {
                                                right_pressed.set(true);
                                                right_flasher.set(Some(spawn(clone!((right_pressed) async move {
                                                    sleep(Duration::from_millis(FLASH_MS as u64)).await;
                                                    right_pressed.signal().wait_for(true).await;
                                                    right_pressed.set(false);
                                                }))));
                                                1
                                            })
                                            as isize
                                        };
                                        i = (i + step + options.lock_ref().len() as isize) % options.lock_ref().len() as isize;
                                        selected.set(options.lock_ref()[i as usize].clone());
                                    }
                                },
                                _ => ()
                            }
                        }),
                        signal::not(controlling_signal)
                    )
                })
                .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING * 2.))
                .item({
                    lil_baby_button()
                    .selected_signal(left_pressed.signal())
                    .on_click(clone!((selected, options) move || {
                        let options_lock = options.lock_ref();
                        if let Some(i) = options_lock.iter().position(|option| option == &*selected.lock_ref()) {
                            selected.set_neq(options_lock.iter().rev().cycle().nth(options_lock.len() - i).unwrap().clone());
                        }
                    }))
                    .body(arrow_text(LeftRight::Left))
                })
                .item(
                    El::<Text>::new()
                    .text_font(TextFont::from_font_size(FONT_SIZE))
                    .text_signal(selected.signal_ref(ToString::to_string).map(Text))
                )
                .item({
                    lil_baby_button()
                    .selected_signal(right_pressed.signal())
                    .on_click(clone!((selected, options) move || {
                        let options_lock = options.lock_ref();
                        if let Some(i) = options_lock.iter().position(|option| option == &*selected.lock_ref()) {
                            selected.set_neq(options_lock.iter().cycle().nth(i + 1).unwrap().clone());
                        }
                    }))
                    .body(arrow_text(LeftRight::Right))
                })
            },
            controlling,
        }
    }
}

impl ElementWrapper for IterableOptions {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for IterableOptions {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
    }
}

struct Slider {
    el: Row<Node>,
    controlling: Mutable<bool>,
}

impl Slider {
    fn new(value: Mutable<f32>) -> Self {
        let (controlling, controlling_signal) = Mutable::new_and_signal(false);
        Self {
            el: {
                let slider_width = 400.;
                let slider_padding = 5.;
                let max = slider_width - slider_padding - LIL_BABY_BUTTON_SIZE - BASE_BORDER_WIDTH;
                let left = Mutable::new(value.get() / 100. * max);
                let value_setter = spawn(clone!((left, value) async move {
                    left.signal().for_each_sync(|left| value.set_neq(left / max * 100.)).await;
                }));
                Row::<Node>::new()
                    .update_raw_el(|raw_el| raw_el.insert(SliderTag))
                    .apply(|element| focus_on_signal(element, controlling.signal()))
                    .update_raw_el(|raw_el| {
                        raw_el.on_event_disableable_signal::<MenuInput>(
                            clone!((left) move |event| {
                                match event {
                                    MenuInput::Left | MenuInput::Right => {
                                        let dir = if matches!(event, MenuInput::Left) { -1. } else { 1. };
                                        left.update(move |left| (left + dir * max * 0.001).max(0.).min(max));
                                    },
                                    _ => ()
                                }
                            }),
                            signal::not(controlling_signal),
                        )
                    })
                    .update_raw_el(|raw_el| raw_el.hold_tasks([value_setter]))
                    .with_node(|mut node| node.column_gap = Val::Px(10.))
                    .item(
                        El::<Text>::new()
                            .text_font(TextFont::from_font_size(FONT_SIZE))
                            .text_signal(value.signal().map(|value| Text(format!("{:.1}", value)))),
                    )
                    .item(
                        Stack::<Node>::new()
                            .width(Val::Px(slider_width))
                            .height(Val::Px(5.))
                            .with_node(move |mut node| node.padding = UiRect::horizontal(Val::Px(slider_padding)))
                            .background_color(BackgroundColor(Color::BLACK))
                            .layer({
                                let dragging = Mutable::new(false);
                                lil_baby_button()
                                    .selected_signal(dragging.signal())
                                    .el // we need lower level access now
                                    .on_signal_with_node(left.signal(), |mut node, left| node.left = Val::Px(left))
                                    .align(Align::new().center_y())
                                    .update_raw_el(|raw_el| {
                                        raw_el
                                            .on_event::<Pointer<DragStart>>(
                                                clone!((dragging) move |_| dragging.set_neq(true)),
                                            )
                                            .on_event::<Pointer<DragEnd>>(move |_| dragging.set_neq(false))
                                            .on_event::<Pointer<Drag>>(move |drag| {
                                                left.set_neq((left.get() + drag.delta.x).max(0.).min(max));
                                            })
                                    })
                            }),
                    )
            },
            controlling,
        }
    }
}

impl ElementWrapper for Slider {
    type EL = Row<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for Slider {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
    }
}

fn options(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("option {}", i)).collect()
}

fn only_one_up_flipper(
    to_flip: &Mutable<bool>,
    already_up_option: &Mutable<Option<Mutable<bool>>>,
    target_option: Option<bool>,
) {
    let cur = target_option.map(|target| !target).unwrap_or(to_flip.get());
    if cur {
        already_up_option.take();
    } else {
        if let Some(previous) = &*already_up_option.lock_ref() {
            previous.set(false);
        }
        already_up_option.set(Some(to_flip.clone()));
    }
    to_flip.set(!cur);
}

static MENU_ITEM_HOVERED_OPTION: Lazy<Mutable<Option<Mutable<bool>>>> = Lazy::new(default);

fn menu_item(label: &str, body: impl Element, hovered: Mutable<bool>) -> Stack<Node> {
    Stack::<Node>::new()
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| NORMAL_BUTTON.lighter(0.05), || NORMAL_BUTTON)
                .map(Into::into),
        )
        .on_hovered_change(move |is_hovered| only_one_up_flipper(&hovered, &MENU_ITEM_HOVERED_OPTION, Some(is_hovered)))
        .width(Val::Percent(100.))
        .height(Val::Px(MENU_ITEM_HEIGHT))
        .with_node(|mut node| node.padding = UiRect::axes(Val::Px(BASE_PADDING), Val::Px(BASE_PADDING / 2.)))
        .layer(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new(label))
                .align(Align::new().left().center_y()),
        )
        .layer(body.align(Align::new().right().center_y()))
}

struct Dropdown {
    el: El<Node>,
    controlling: Mutable<bool>,
}

fn focus_on_signal<E: Element>(element: E, signal: impl Signal<Item = bool> + Send + 'static) -> E {
    element.update_raw_el(|raw_el| {
        raw_el.on_signal(signal.dedupe(), |entity, focus| async move {
            if focus {
                // at first, i was using a `static_ref` global `Mutable<Option<Entity>>` for this
                // and wrapping it in a resource for accessing it in the menu input event systems, but this is an
                // anti pattern; the ecs should not be polling reactive ui state for syncing its own
                // state/systems; instead, like we do here, simply use the `async_world` to update the ecs state
                // *exactly and only* when it needs to be
                async_world().insert_resource(FocusedEntity(entity)).await;
            }
        })
    })
}

impl Dropdown {
    fn new<T: Clone + PartialEq + Display + Send + Sync + 'static>(
        options: MutableVec<T>,
        selected: Mutable<Option<T>>,
        clearable: bool,
    ) -> Self {
        let show_dropdown = Mutable::new(false);
        let hovered = Mutable::new(false);
        let controlling = Mutable::new(false);
        let options_hovered =
            MutableVec::new_with_values((0..options.lock_ref().len()).map(|_| Mutable::new(false)).collect());
        let el = {
            El::<Node>::new()
            .apply(|element| focus_on_signal(element, controlling.signal()))
            .update_raw_el(|raw_el| {
                raw_el.observe::<MenuInput, _, _>(
                    clone!((controlling, show_dropdown, hovered, options, options_hovered, selected) move |mut event: Trigger<MenuInput>| {
                        // TODO: this is cringe, but the component driven alternative is equally cringe ? (need to use .observe here directly since we need to stop propagation conditionally within the body of the callback)
                        if controlling.get() {
                            match *event {
                                MenuInput::Up | MenuInput::Down => {
                                    if show_dropdown.get() {
                                        event.propagate(false);
                                        let hovered_option = options_hovered.lock_ref().iter().position(|hovered| hovered.get());
                                        if let Some(i) = hovered_option {
                                            options_hovered.lock_ref()[i].set(false);
                                        }
                                        let (mut i, step) = {
                                            if matches!(*event, MenuInput::Up) {
                                                (hovered_option.unwrap_or(options.lock_ref().len() - 1) as isize, -1)
                                            } else {
                                                (hovered_option.unwrap_or(0) as isize, 1)
                                            }
                                        };
                                        if hovered_option.is_some() || (selected.lock_ref().is_some() && Some(&options.lock_ref()[i as usize]) == selected.lock_ref().as_ref()) {
                                            for _ in 0..options.lock_ref().len() {
                                                i = (i + step + options.lock_ref().len() as isize) % options.lock_ref().len() as isize;
                                                if Some(&options.lock_ref()[i as usize]) != selected.lock_ref().as_ref() {
                                                    break;
                                                }
                                            }
                                        }
                                        options_hovered.lock_ref()[i as usize].set(true);
                                    } else {
                                        hovered.set_neq(false);
                                    }
                                }
                                MenuInput::Select => {
                                    hovered.set_neq(!show_dropdown.get());
                                    let hovered_option = options_hovered.lock_ref().iter().position(|hovered| hovered.get());
                                    if let Some(i) = hovered_option {
                                        options_hovered.lock_ref()[i].set(false);
                                        selected.set_neq(Some(options.lock_ref()[i].clone()));
                                    }
                                    flip(&show_dropdown);
                                    for hovered in options_hovered.lock_ref().iter() {
                                        hovered.set(false);
                                    }
                                },
                                MenuInput::Back => {
                                    if show_dropdown.get() {
                                        event.propagate(false);
                                        for hovering in options_hovered.lock_ref().iter() {
                                            hovering.set(false);
                                        }
                                        flip(&show_dropdown);
                                    }
                                    hovered.set(false);
                                },
                                MenuInput::Delete => {
                                    if clearable {
                                        selected.take();
                                    }
                                },
                                _ => ()
                            }
                        }
                    }),
                )
            })
            .child(
                Button::new()
                .width(Val::Px(300.))
                .hovered_signal(hovered.signal())
                .body(
                    Stack::<Node>::new()
                    .width(Val::Percent(100.))
                    .with_node(|mut node| node.padding = UiRect::horizontal(Val::Px(BASE_PADDING)))
                    .layer(
                        El::<Text>::new()
                        .align(Align::new().left())
                        .text_font(TextFont::from_font_size(FONT_SIZE))
                        .text_signal(
                            selected.signal_cloned()
                            .map(|selected_option| {
                                selected_option.map(|option| option.to_string()).unwrap_or_default()
                            })
                            .map(Text)
                        )
                    )
                    .layer(
                        Row::<Node>::new()
                        .with_node(|mut node| node.column_gap = Val::Px(BASE_PADDING))
                        .align(Align::new().right())
                        .item_signal(
                            // TODO: this should just work, but compiler asks for type info
                            // clearable.then(||
                            //     selected.signal_ref(Option::is_some).dedupe()
                            //     .map_true(clone!((selected) move || x_button(clone!((selected) move || { selected.take(); }))))
                            // )
                            if clearable {
                                selected.signal_ref(Option::is_some).dedupe()
                                .map_true(clone!((selected) move || x_button(clone!((selected) move || { selected.take(); }))))
                                .boxed()
                            } else {
                                always(None).boxed()
                            }
                        )
                        .item(
                            El::<Text>::new()
                            .text_font(TextFont::from_font_size(FONT_SIZE))
                            // TODO: need to figure out to rotate in place (around center)
                            // .on_signal_with_transform(show_dropdown.signal(), |transform, showing| {
                            //     transform.rotate_around(Vec3::X, Quat::from_rotation_z((if showing { 180.0f32 } else { 0. }).to_radians()));
                            // })
                            .text(Text::new("v"))
                        )
                    )
                )
                .on_click(clone!((show_dropdown) move || {
                    only_one_up_flipper(&show_dropdown, &DROPDOWN_SHOWING_OPTION, None);
                }))
            )
            // TODO: this should be element below signal
            .child_signal(
                show_dropdown.signal()
                .map_true(clone!((options, show_dropdown, selected) move || {
                    Column::<Node>::new()
                    .width(Val::Percent(100.))
                    .with_node(|mut node| {
                        node.position_type = PositionType::Absolute;
                        node.top = Val::Percent(100.);
                    })
                    .items_signal_vec(
                        options.signal_vec_cloned()
                        .enumerate()
                        .filter_signal_cloned(clone!((selected) move |(_, option)| {
                            selected.signal_ref(clone!((option) move |selected_option| {
                                selected_option.as_ref() != Some(&option)
                            }))
                            .dedupe()
                        }))
                        .map_signal(clone!((selected, show_dropdown, options_hovered) move |(i_mutable, option)| {
                            i_mutable.signal()
                            .map_some(clone!((options_hovered, selected, show_dropdown, option) move |i| {
                                if let Some(hovered) = options_hovered.lock_ref().get(i) {
                                    text_button(
                                        always(option.to_string()),
                                        clone!((selected, show_dropdown, option) move || {
                                            selected.set_neq(Some(option.clone()));
                                            flip(&show_dropdown);
                                        })
                                    )
                                    .width(Val::Percent(100.))
                                    .hovered_signal(hovered.signal())
                                    .apply(Some)
                                } else {
                                    None
                                }
                            }))
                        }))
                        .map(Option::flatten)
                    )
                }))
            )
        };
        Self { el, controlling }
    }
}

impl ElementWrapper for Dropdown {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for Dropdown {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
    }
}

fn focus_on_no_child_hovered<E: Element>(
    element: E,
    hovereds: impl SignalVec<Item = Mutable<bool>> + Send + 'static,
) -> E {
    focus_on_signal(element, {
        hovereds
            .map_signal(|hovered| hovered.signal())
            .to_signal_map(|is_hovereds| !is_hovereds.iter().copied().any(identity))
            .dedupe()
    })
}

fn sub_menu_child_hover_manager<E: Element>(element: E, hovereds: MutableVec<Mutable<bool>>) -> E {
    let l = hovereds.lock_ref().len();
    element.update_raw_el(|raw_el| {
        raw_el.on_event::<MenuInput>(clone!((hovereds) move |event| {
            let hovereds_lock = hovereds.lock_ref();
            match event {
                MenuInput::Up | MenuInput::Down => {
                    let hovered_option = hovereds_lock.iter().position(|hovered| hovered.get());
                    if let Some(i) = hovered_option {
                        hovereds_lock[i].set(false);
                        let new_i = if matches!(event, MenuInput::Up) { i + l - 1 } else { i + 1 } % l;
                        hovereds_lock[new_i].set(true);
                    } else {
                        let i = if matches!(event, MenuInput::Up) { hovereds_lock.len() - 1 } else { 0 };
                        hovereds_lock[i].set(true);
                    }
                },
                MenuInput::Back => {
                    if hovereds_lock.iter().any(|hovered| hovered.get()) {
                        for hovered in hovereds_lock.iter() {
                            hovered.set(false)
                        }
                    } else {
                        SHOW_SUB_MENU.set(None);
                    }
                },
                _ => ()
            }
        }))
    })
}

fn make_controlling_menu_item(label: &str, el: impl Controllable + Element) -> (Stack<Node>, Mutable<bool>) {
    let hovered = Mutable::new(false);
    (
        menu_item(label, el.controlling_signal(hovered.signal()), hovered.clone()),
        hovered,
    )
}

fn audio_menu() -> Column<Node> {
    let items_hovereds = [
        make_controlling_menu_item(
            "dropdown",
            Dropdown::new(
                MutableVec::new_with_values(options(4)),
                MISC_DEMO_SETTINGS.dropdown.clone(),
                true,
            ),
        ),
        make_controlling_menu_item(
            "radio group",
            RadioGroup::new(
                MutableVec::new_with_values(options(3)),
                MISC_DEMO_SETTINGS.radio_group.clone(),
            ),
        ),
        make_controlling_menu_item("checkbox", Checkbox::new(MISC_DEMO_SETTINGS.checkbox.clone())),
        make_controlling_menu_item(
            "iterable options",
            IterableOptions::new(
                MutableVec::new_with_values(options(4)),
                MISC_DEMO_SETTINGS.iterable_options.clone(),
            ),
        ),
        make_controlling_menu_item("master volume", Slider::new(AUDIO_SETTINGS.master_volume.clone())),
        make_controlling_menu_item("effect volume", Slider::new(AUDIO_SETTINGS.effect_volume.clone())),
        make_controlling_menu_item("music volume", Slider::new(AUDIO_SETTINGS.music_volume.clone())),
        make_controlling_menu_item("voice volume", Slider::new(AUDIO_SETTINGS.voice_volume.clone())),
    ];
    let l = items_hovereds.len();
    let (items, hovereds): (Vec<_>, Vec<_>) = items_hovereds.into_iter().unzip();
    let hovereds = MutableVec::new_with_values(hovereds);
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "audio menu")
        .apply(|element| focus_on_no_child_hovered(element, hovereds.signal_vec_cloned()))
        .apply(|element| sub_menu_child_hover_manager(element, hovereds.clone()))
        .items(
            items
                .into_iter()
                .enumerate()
                .map(move |(i, item)| item.z_index(ZIndex((l - i) as i32))),
        )
}

fn graphics_menu() -> Column<Node> {
    let preset_quality = GRAPHICS_SETTINGS.preset_quality.clone();
    let texture_quality = GRAPHICS_SETTINGS.texture_quality.clone();
    let shadow_quality = GRAPHICS_SETTINGS.shadow_quality.clone();
    let bloom_quality = GRAPHICS_SETTINGS.bloom_quality.clone();
    let non_preset_qualities = MutableVec::new_with_values(vec![
        texture_quality.clone(),
        shadow_quality.clone(),
        bloom_quality.clone(),
    ]);
    let preset_broadcaster = spawn(clone!((preset_quality, non_preset_qualities) async move {
        preset_quality.signal()
        .for_each_sync(|preset_quality_option| {
            if let Some(preset_quality) = preset_quality_option {
                for quality in non_preset_qualities.lock_ref().iter() {
                    quality.set_neq(Some(preset_quality));
                }
            }
        })
        .await;
    }));
    let preset_controller = spawn(clone!((preset_quality) async move {
        non_preset_qualities.signal_vec_cloned()
        .map_signal(|quality| quality.signal())
        .to_signal_map(|qualities| {
            let mut qualities = qualities.iter();
            let mut preset = preset_quality.lock_mut();
            if preset.is_none() {
                let first = qualities.next().unwrap();  // always populated
                if qualities.all(|quality| quality == first) {
                    *preset = *first;
                }
            } else if preset.is_some() && qualities.any(|quality| quality != &*preset) {
                *preset = None;
            }
        })
        .to_future()
        .await;
    }));
    let items = [
        ("preset quality", preset_quality, true),
        ("texture quality", texture_quality, false),
        ("shadow quality", shadow_quality, false),
        ("bloom quality", bloom_quality, false),
    ];
    let l = items.len();
    let hovereds = MutableVec::new_with_values((0..l).map(|_| Mutable::new(false)).collect::<Vec<_>>());
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "graphics menu")
        .apply(|element| focus_on_no_child_hovered(element, hovereds.signal_vec_cloned()))
        .apply(|element| sub_menu_child_hover_manager(element, hovereds.clone()))
        .update_raw_el(|raw_el| raw_el.hold_tasks([preset_broadcaster, preset_controller]))
        .items({
            let hovereds = hovereds.lock_ref().iter().cloned().collect::<Vec<_>>();
            items
                .into_iter()
                .zip(hovereds)
                .enumerate()
                .map(move |(i, ((label, quality, clearable), hovered))| {
                    menu_item(
                        label,
                        {
                            Dropdown::new(
                                MutableVec::new_with_values(Quality::iter().collect()),
                                quality,
                                clearable,
                            )
                            .controlling_signal(hovered.signal())
                        },
                        hovered,
                    )
                    .z_index(ZIndex((l - i) as i32))
                })
        })
        .item(
            // solely here to dehover dropdown menu items  // TODO: this can also be solved by
            // allowing setting Over/Out order at runtime or implementing .on_hovered_outside, i
            // should do both of these
            El::<Node>::new()
                .height(Val::Px(
                    SUB_MENU_HEIGHT - (l + 1) as f32 * MENU_ITEM_HEIGHT - BASE_PADDING * 2.,
                ))
                .on_hovered_change(|is_hovered| {
                    if is_hovered {
                        if let Some(hovered) = MENU_ITEM_HOVERED_OPTION.take() {
                            hovered.set(false);
                        }
                    }
                }),
        )
}

fn x_button(on_click: impl FnMut() + Send + Sync + 'static) -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .background_color(BackgroundColor(Color::NONE))
        .hovered_sync(hovered.clone())
        // stop propagation because otherwise clearing the dropdown will drop down the
        // options too; the x should eat the click
        .on_click_stop_propagation(on_click)
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(FONT_SIZE))
                .text(Text::new("x"))
                .text_color_signal(
                    hovered
                        .signal()
                        .map_bool(|| bevy::color::palettes::basic::RED.into(), || TEXT_COLOR)
                        .map(Into::into),
                ),
        )
}

static SUB_MENU_SELECTED: Lazy<Mutable<Option<SubMenu>>> = Lazy::new(default);

static SHOW_SUB_MENU: Lazy<Mutable<Option<SubMenu>>> = Lazy::new(default);

fn menu() -> impl Element {
    Stack::<Node>::new()
        .layer(
            menu_base(MAIN_MENU_SIDES, MAIN_MENU_SIDES, "main menu")
                .apply(|element| focus_on_signal(element, SHOW_SUB_MENU.signal_ref(Option::is_none)))
                .update_raw_el(|raw_el| {
                    raw_el.on_event_disableable_signal::<MenuInput>(
                        move |event| match event {
                            MenuInput::Up | MenuInput::Down => {
                                if let Some(cur_sub_menu) = SUB_MENU_SELECTED.get() {
                                    if let Some(i) = SubMenu::iter().position(|sub_menu| cur_sub_menu == sub_menu) {
                                        let sub_menus = SubMenu::iter().collect::<Vec<_>>();
                                        SUB_MENU_SELECTED.set(if matches!(event, MenuInput::Down) {
                                            sub_menus.iter().rev().cycle().nth(sub_menus.len() - i).copied()
                                        } else {
                                            sub_menus.iter().cycle().nth(i + 1).copied()
                                        })
                                    }
                                } else {
                                    SUB_MENU_SELECTED.set_neq(Some(if matches!(event, MenuInput::Up) {
                                        SubMenu::iter().last().unwrap()
                                    } else {
                                        SubMenu::iter().next().unwrap()
                                    }));
                                }
                            }
                            MenuInput::Select => {
                                if let Some(sub_menu) = SUB_MENU_SELECTED.get() {
                                    SHOW_SUB_MENU.set_neq(Some(sub_menu));
                                }
                            }
                            MenuInput::Back => {
                                SUB_MENU_SELECTED.take();
                            }
                            _ => (),
                        },
                        SHOW_SUB_MENU.signal_ref(Option::is_some),
                    )
                })
                .with_node(|mut node| node.row_gap = Val::Px(BASE_PADDING * 2.))
                .item(
                    Column::<Node>::new()
                        .with_node(|mut node| node.row_gap = Val::Px(BASE_PADDING))
                        .align_content(Align::center())
                        .items(SubMenu::iter().map(|sub_menu| {
                            sub_menu_button(sub_menu).hovered_signal(
                                SUB_MENU_SELECTED.signal_ref(move |selected_option| selected_option == &Some(sub_menu)),
                            )
                        })),
                ),
        )
        .layer_signal(SHOW_SUB_MENU.signal().map_some(move |sub_menu| {
            let menu = match sub_menu {
                SubMenu::Audio => audio_menu(),
                SubMenu::Graphics => graphics_menu(),
            };
            Stack::<Node>::new()
                .width(Val::Px(SUB_MENU_WIDTH))
                .height(Val::Px(SUB_MENU_HEIGHT))
                .with_node(|mut node| {
                    // TODO: without absolute there's some weird bouncing when switching between
                    // menus, perhaps due to the layout system having to figure stuff out ?
                    node.position_type = PositionType::Absolute;
                })
                .align(Align::center())
                .layer(menu.align(Align::center()))
                .layer(
                    x_button(|| {
                        SHOW_SUB_MENU.take();
                    })
                    .align(Align::new().top().right())
                    .update_raw_el(|raw_el| {
                        raw_el.with_component::<Node>(|mut node| {
                            node.padding.right = Val::Px(BASE_PADDING);
                            node.padding.top = Val::Px(BASE_PADDING / 2.);
                        })
                    }),
                )
        }))
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Resource, Clone)]
struct AudioSettings {
    master_volume: Mutable<f32>,
    effect_volume: Mutable<f32>,
    music_volume: Mutable<f32>,
    voice_volume: Mutable<f32>,
}

static AUDIO_SETTINGS: Lazy<AudioSettings> = Lazy::new(|| AudioSettings {
    master_volume: Mutable::new(100.),
    effect_volume: Mutable::new(50.),
    music_volume: Mutable::new(50.),
    voice_volume: Mutable::new(50.),
});

#[derive(Resource, Clone)]
struct GraphicsSettings {
    preset_quality: Mutable<Option<Quality>>,
    texture_quality: Mutable<Option<Quality>>,
    shadow_quality: Mutable<Option<Quality>>,
    bloom_quality: Mutable<Option<Quality>>,
}

static GRAPHICS_SETTINGS: Lazy<GraphicsSettings> = Lazy::new(|| GraphicsSettings {
    preset_quality: Mutable::new(Some(Quality::Medium)),
    texture_quality: Mutable::new(Some(Quality::Medium)),
    shadow_quality: Mutable::new(Some(Quality::Medium)),
    bloom_quality: Mutable::new(Some(Quality::Medium)),
});

#[derive(Resource, Clone)]
struct MiscDemoSettings {
    dropdown: Mutable<Option<String>>,
    radio_group: Mutable<Option<usize>>,
    checkbox: Mutable<bool>,
    iterable_options: Mutable<String>,
}

static MISC_DEMO_SETTINGS: Lazy<MiscDemoSettings> = Lazy::new(|| MiscDemoSettings {
    dropdown: Mutable::new(None),
    radio_group: Mutable::new(None),
    checkbox: Mutable::new(false),
    iterable_options: Mutable::new("option 1".to_string()),
});

#[derive(Clone, Copy, Component)]
enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Delete,
}

impl Event for MenuInput {
    type Traversal = &'static Parent;

    const AUTO_PROPAGATE: bool = true;
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
            if rate_limiter.tick(time.delta()).finished() {
                commands.trigger_targets(input, entity);
                rate_limiter.reset();
            }
            true
        }
        PressedType::JustPressed => {
            commands.trigger_targets(input, entity);
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
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(menu())
}
