// Main menu with sub menus for audio and graphics.
// Simple buttons for option selection.
// Slider for volume.
// Dropdown for graphics quality (low/medium/high).
// Navigation possible with mouse, keyboard and controller.
//     Mouse: Separate styles for hover and press.
//     Keyboard/Controller: Separate styles for currently focused element.

use std::{fmt::Display, convert::identity};

use bevy::{prelude::*, utils::tracing::field::Iter};
use haalka::*;
use futures_signals::map_ref;
use strum::{Display, EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin
        ))
        .add_plugins(EventListenerPlugin::<MenuInputEvent>::default())
        .add_systems(Startup, (setup, spawn_ui_root))
        .add_systems(Update, keyboard_events)
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const FONT_SIZE: f32 = 30.;
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

struct Button {
    el: El<NodeBundle>,
    selected: Mutable<bool>,
    hovered: Mutable<bool>,
}

impl ElementWrapper for Button {
    type EL = El<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Button {
    fn new() -> Self {
        let (selected, selected_signal) = Mutable::new_and_signal(false);
        let (hovered, hovered_signal) = Mutable::new_and_signal(false);
        let selected_hovered_broadcaster = map_ref!(selected_signal, hovered_signal => (*selected_signal, *hovered_signal)).broadcast();
        let border_color_signal = {
            selected_hovered_broadcaster.signal()
            .map(|(selected, hovered)| {
                if selected { Color::RED } else if hovered { Color::WHITE } else { Color::BLACK }
            })
            .map(BorderColor)
        };
        let background_color_signal = {
            selected_hovered_broadcaster.signal()
            .map(|(selected, hovered)| {
                if selected { CLICKED_BUTTON } else if hovered { HOVERED_BUTTON } else { NORMAL_BUTTON }
            })
            .map(BackgroundColor)
        };
        Self {
            el: {
                El::<NodeBundle>::new()
                .with_style(move |style| {
                    style.height = Val::Px(DEFAULT_BUTTON_HEIGHT);
                    style.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                })
                .align_content(Align::center())
                .hovered_sync(hovered.clone())
                .border_color_signal(border_color_signal)
                .background_color_signal(background_color_signal)
            },
            selected,
            hovered,
        }
    }

    fn width(mut self, width: Val) -> Self {
        self.el = self.el.with_style(move |style| {
            style.width = width;
        });
        self
    }

    fn height(mut self, height: Val) -> Self {
        self.el = self.el.with_style(move |style| {
            style.height = height;
        });
        self
    }

    fn body(mut self, body: impl Element) -> Self {
        self.el = self.el.child(body);
        self
    }

    fn on_click(mut self, on_click: impl FnMut() + 'static + Send + Sync) -> Self {
        self.el = self.el.on_click(on_click);
        self
    }

    fn selected_signal(mut self, selected_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let syncer = spawn(sync(self.selected.clone(), selected_signal));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }

    fn hovered_signal(mut self, hovered_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let syncer = spawn(sync(self.hovered.clone(), hovered_signal));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }
}

async fn sync<T>(mutable: Mutable<T>, signal: impl Signal<Item = T> + Send + 'static) {
    signal.for_each_sync(|value| mutable.set(value)).await;
}

fn text(text: &str) -> Text {
    Text::from_section(text, TextStyle { font_size: FONT_SIZE, ..default() })
}

fn text_button(text_signal: impl Signal<Item = String> + Send + 'static, on_click: impl FnMut() + 'static + Send + Sync) -> Button {
    Button::new()
    .width(Val::Px(200.))
    .body(El::<TextBundle>::new().text_signal(text_signal.map(|t| text(&t))))
    .on_click(on_click)
}

fn sub_menu_button(sub_menu: SubMenu) -> Button {
    text_button(
        always(sub_menu.to_string()),
        move || show_sub_menu().set_neq(Some(sub_menu))
    )
}

fn menu_base(width: f32, height: f32, title: &str) -> Column<NodeBundle> {
    Column::<NodeBundle>::new()
    .with_style(move |style| {
        style.width = Val::Px(width);
        style.height = Val::Px(height);
        style.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
    })
    .border_color(BorderColor(Color::BLACK))
    .background_color(BackgroundColor(NORMAL_BUTTON))
    .item(
        El::<NodeBundle>::new()
        .with_style(|style| {
            style.height = Val::Px(MENU_ITEM_HEIGHT);
            style.padding = UiRect::all(Val::Px(BASE_PADDING * 2.));
        })
        .child(
            El::<TextBundle>::new()
            .align(Align::new().top().left())
            .text(text(title))
        )
    )
}

fn flip(mutable_bool: &Mutable<bool>) {
    mutable_bool.set(!mutable_bool.get());
}

#[static_ref]
fn dropdown_showing_option() -> &'static Mutable<Option<Mutable<bool>>> {
    Mutable::new(None)
}

fn lil_baby_button() -> Button {
    Button::new()
    .width(Val::Px(30.))
    .height(Val::Px(30.))
}

struct Checkbox {
    el: Button,
    controlling: Mutable<bool>,
}

impl Checkbox {
    fn new(checked: Mutable<bool>) -> Self {
        let controlling = Mutable::new(false);
        Self {
            el: {
                lil_baby_button()
                .on_click(clone!((checked) move || { flip(&checked) }))
                .selected_signal(checked.signal())
                .into_element()
            },
            controlling
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

fn signal_eq<T: PartialEq + Send>(signal1: impl Signal<Item = T> + Send + 'static, signal2: impl Signal<Item = T> + Send + 'static) -> impl Signal<Item = bool> + Send + 'static {
    map_ref!(signal1, signal2 => *signal1 == *signal2).dedupe()
}

struct MutuallyExclusiveOptions {
    el: Row<NodeBundle>,
    controlling: Mutable<bool>,
}

impl MutuallyExclusiveOptions {
    fn new<T: Clone + PartialEq + Display + Send + Sync + 'static>(options: MutableVec<T>, selected: Mutable<Option<usize>>) -> Self {
        let controlling = Mutable::new(false);
        Self {
            el: {
                Row::<NodeBundle>::new()
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
                        .selected_signal(signal_eq(selected.signal_cloned(), i_option_mutable.signal_cloned()))
                    }))
                )
            },
            controlling,
        }
    }
}

impl ElementWrapper for MutuallyExclusiveOptions {
    type EL = Row<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for MutuallyExclusiveOptions {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
    }
}

enum LeftRight {
    Left,
    Right,
}

fn centered_arrow_text(direction: LeftRight) -> El<TextBundle> {
    El::<TextBundle>::new()
    .with_style(|style| {
        style.bottom = Val::Px(2.);
        style.right = Val::Px(2.);
    })
    .text(text(match direction { LeftRight::Left => "<", LeftRight::Right => ">" }))
}

struct IterableOptions {
    el: Row<NodeBundle>,
    controlling: Mutable<bool>,
}

impl IterableOptions {
    fn new<T: Clone + PartialEq + Display + Send + Sync + 'static>(options: Vec<T>, selected: Mutable<T>) -> impl Element {
        Row::<NodeBundle>::new()
        .with_style(|style| style.column_gap = Val::Px(BASE_PADDING * 2.))
        .item({
            let pressed = Mutable::new(false);
            lil_baby_button()
            .selected_signal(pressed.signal())
            .pressed_sync(pressed)
            .on_click(clone!((selected, options) move || {
                if let Some(i) = options.iter().position(|option| option == &*selected.lock_ref()) {
                    selected.set_neq(options.iter().rev().cycle().skip(options.len() - i).next().unwrap().clone());
                }
            }))
            .body(centered_arrow_text(LeftRight::Left))
        })
        .item(
            El::<TextBundle>::new()
            .text_signal(selected.signal_cloned().map(|selected| text(&selected.to_string())))
        )
        .item({
            let pressed = Mutable::new(false);
            lil_baby_button()
            .selected_signal(pressed.signal())
            .pressed_sync(pressed)
            .on_click(clone!((selected, options) move || {
                if let Some(i) = options.iter().position(|option| option == &*selected.lock_ref()) {
                    selected.set_neq(options.iter().cycle().skip(i + 1).next().unwrap().clone());
                }
            }))
            .body(centered_arrow_text(LeftRight::Right))
        })
    }
}

impl ElementWrapper for IterableOptions {
    type EL = Row<NodeBundle>;
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
    el: Row<NodeBundle>,
    controlling: Mutable<bool>,
}

impl Slider {
    fn new(value: Mutable<f32>) -> Self {
        let controlling = Mutable::new(false);
        Self {
            el: {
                let slider_width = 400.;
                let slider_padding = 5.;
                let max = slider_width - slider_padding - LIL_BABY_BUTTON_SIZE - BASE_BORDER_WIDTH;
                let left = Mutable::new(value.get() / 100. * max);
                let value_setter = spawn(clone!((left, value) async move {
                    left.signal().for_each_sync(|left| value.set_neq(left / max * 100.)).await;
                }));
                Row::<NodeBundle>::new()
                .update_raw_el(|raw_el| raw_el.hold_tasks([value_setter]))
                .with_style(|style| style.column_gap = Val::Px(10.))
                .item(
                    El::<TextBundle>::new()
                    .text_signal(value.signal().map(|value| text(&format!("{:.1}", value))))
                )
                .item(
                    Stack::<NodeBundle>::new()
                    .with_style(move |style| {
                        style.width = Val::Px(slider_width);
                        style.height = Val::Px(5.);
                        style.padding = UiRect::horizontal(Val::Px(slider_padding));
                    })
                    .background_color(BackgroundColor(Color::BLACK))
                    .layer({
                        let dragging = Mutable::new(false);
                        let pressed = Mutable::new(false);
                        lil_baby_button()
                        .selected_signal(signal::or(pressed.signal(), dragging.signal()))
                        .pressed_sync(pressed)
                        .el  // we need lower level access now
                        .on_signal_with_style(left.signal(), |style, left| style.left = Val::Px(left))
                        .align(Align::new().center_y())
                        .update_raw_el(|raw_el| {
                            raw_el
                            .insert((
                                On::<Pointer<DragStart>>::run(clone!((dragging) move || dragging.set_neq(true))),
                                On::<Pointer<DragEnd>>::run(move || dragging.set_neq(false)),
                                On::<Pointer<Drag>>::run(move |drag: Listener<Pointer<Drag>>| {
                                    left.set_neq((left.get() + drag.delta.x).max(0.).min(max));
                                }),
                            ))
                        })
                    })
                )
            },
            controlling,
        }
    }
}

impl ElementWrapper for Slider {
    type EL = Row<NodeBundle>;
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

fn only_one_up_flipper<'a>(
    to_flip: &Mutable<bool>,
    already_up_option: &'a Mutable<Option<Mutable<bool>>>,
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

#[static_ref]
fn menu_item_hovered_option() -> &'static Mutable<Option<Mutable<bool>>> {
    Mutable::new(None)
}

fn menu_item(label: &str, body: impl Element, hovered: Mutable<bool>) -> Stack<NodeBundle> {
    Stack::<NodeBundle>::new()
    .background_color_signal(
        hovered.signal()
        .map_bool(
            || NORMAL_BUTTON.with_l(NORMAL_BUTTON.l() + 0.1),
            || NORMAL_BUTTON,
        )
        .map(BackgroundColor)
    )
    .on_hovered_change(move |is_hovered| {
        only_one_up_flipper(&hovered, menu_item_hovered_option(), Some(is_hovered));
    })
    .with_style(|style| {
        style.width = Val::Percent(100.);
        style.padding = UiRect::axes(Val::Px(BASE_PADDING), Val::Px(BASE_PADDING / 2.));
        style.height = Val::Px(MENU_ITEM_HEIGHT);
    })
    .layer(El::<TextBundle>::new().text(text(label)).align(Align::new().left().center_y()))
    .layer(body.align(Align::new().right().center_y()))
}

trait Controllable: ElementWrapper where Self: Sized + 'static {
    fn controlling(&self) -> &Mutable<bool>;

    fn controlling_signal(mut self, controlling_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let syncer = spawn(sync(self.controlling().clone(), controlling_signal));
        self = self.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }
}

struct Dropdown {
    el: El<NodeBundle>,
    controlling: Mutable<bool>,
}

impl Dropdown {
    fn new<T: Clone + PartialEq + Display + Send + Sync + 'static>(options: MutableVec<T>, selected: Mutable<Option<T>>, clearable: bool) -> Self {
        let show_dropdown = Mutable::new(false);
        let hovered = Mutable::new(false);
        let pressed = Mutable::new(false);
        let controlling = Mutable::new(false);
        let options_hovered = MutableVec::new_with_values((0..options.lock_ref().len()).into_iter().map(|_| Mutable::new(false)).collect());
        let el = {
            El::<NodeBundle>::new()
            .update_raw_el(|raw_el|
                raw_el
                .on_signal_sync(
                    controlling.signal(),
                    |entity, controlling| {
                        if controlling {
                            entity_focused_option().set(Some(entity));
                        }
                    }
                )
            )
            .apply(|element| {
                input_event_listener_controller(
                    element,
                    controlling.signal(),
                    clone!((show_dropdown, hovered, options, options_hovered) move || {
                        On::<MenuInputEvent>::run(clone!((show_dropdown, hovered, options, options_hovered) move |mut event: ListenerMut<MenuInputEvent>| {
                            match event.input {
                                MenuInput::Up | MenuInput::Down => {
                                    if show_dropdown.get() {
                                        event.stop_propagation();
                                        let hovered_option = options_hovered.lock_ref().iter().position(|hovered| hovered.get());
                                        // TODO next: must skip selected option; store selected/hovered in the option itself ?
                                        if let Some(i) = hovered_option {
                                            let new_i = if matches!(event.input, MenuInput::Up) { i + options.lock_ref().len() - 1 } else { i + 1 } % options.lock_ref().len();
                                            options_hovered.lock_ref()[i].set(false);
                                            options_hovered.lock_ref()[new_i].set(true);
                                        } else {
                                            options_hovered.lock_ref()[if matches!(event.input, MenuInput::Up) { options.lock_ref().len() - 1 } else { 0 }].set(true);
                                        }
                                    } else {
                                        hovered.set_neq(false);
                                    }
                                }
                                MenuInput::Select => {
                                    if !hovered.get() {
                                        hovered.set(true);
                                    } else {
                                        flip(&show_dropdown);
                                    }
                                },
                                MenuInput::Back => {
                                    if hovered.get() {
                                        event.stop_propagation();
                                        if show_dropdown.get() {
                                            flip(&show_dropdown);
                                        } else {
                                            hovered.set(false);
                                        }
                                    }
                                },
                                _ => ()
                            }
                        }))
                    })
                )
            })
            .child(
                Button::new()
                .width(Val::Px(300.))
                .hovered_signal(hovered.signal())
                .selected_signal(pressed.signal())
                .pressed_sync(pressed)
                .body(
                    Stack::<NodeBundle>::new()
                    .with_style(|style| {
                        style.width = Val::Percent(100.);
                        style.padding = UiRect::horizontal(Val::Px(BASE_PADDING));
                    })
                    .layer(
                        El::<TextBundle>::new()
                        .align(Align::new().left())
                        .text_signal(
                            selected.signal_cloned()
                            .map(|selected_option| {
                                selected_option.map(|option| option.to_string()).unwrap_or_default()
                            })
                            .map(|t| text(&t))
                        )
                    )
                    .layer(
                        Row::<NodeBundle>::new()
                        .with_style(|style| style.column_gap = Val::Px(BASE_PADDING))
                        .align(Align::new().right())
                        .item_signal({
                            if clearable {
                                selected.signal_ref(Option::is_some).dedupe()
                                .map_true(clone!((selected) move || x_button(clone!((selected) move || { selected.take(); }))))
                                .boxed()
                            } else {
                                always(None).boxed()
                            }
                        })
                        .item(
                            El::<TextBundle>::new()
                            // TODO: need to figure out to rotate in place (around center)
                            // .on_signal_with_transform(show_dropdown.signal(), |transform, showing| {
                            //     transform.rotate_around(Vec3::X, Quat::from_rotation_z((if showing { 180.0f32 } else { 0. }).to_radians()));
                            // })
                            .text(text("v"))
                        )
                    )
                )
                .on_click(clone!((show_dropdown) move || {
                    only_one_up_flipper(&show_dropdown, dropdown_showing_option(), None);
                }))
            )
            .child_signal(
                show_dropdown.signal()
                .map_true(clone!((options, show_dropdown, selected) move || {
                    Column::<NodeBundle>::new()
                    .with_style(|style| {
                        style.position_type = PositionType::Absolute;
                        style.top = Val::Percent(100.);
                        style.width = Val::Percent(100.);
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
                                    let pressed = Mutable::new(false);
                                    text_button(
                                        always(option.to_string()),
                                        clone!((selected, show_dropdown, option) move || {
                                            selected.set_neq(Some(option.clone()));
                                            flip(&show_dropdown);
                                        })
                                    )
                                    .width(Val::Percent(100.))
                                    .hovered_signal(hovered.signal())
                                    .selected_signal(pressed.signal())
                                    .pressed_sync(pressed)
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
        Self {
            el,
            controlling,
        }
    }
}

impl ElementWrapper for Dropdown {
    type EL = El<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Controllable for Dropdown {
    fn controlling(&self) -> &Mutable<bool> {
        &self.controlling
    }
}

fn audio_menu() -> Column<NodeBundle> {
    let item_funcs = [
        // TODO: figure out a way to type erase so vecs of diff widgets can be constructed ... MoonZoon's .into_raw_element strat doesn't work because NodeBuilder isn't type erased; this wouldn't be an issue if there was only a single node type
        // i could just manually count the menu items at compile time, but we want this to be dynamic so it's easy to add new items
        |hovered: Mutable<bool>| menu_item("item 1", Dropdown::new(MutableVec::new_with_values(options(4)), misc_demo_settings().dropdown.clone(), true).controlling_signal(hovered.signal()), hovered),
        |hovered: Mutable<bool>| menu_item("item 2", MutuallyExclusiveOptions::new(MutableVec::new_with_values(options(3)), misc_demo_settings().mutually_exclusive_options.clone()).controlling_signal(hovered.signal()), hovered),
        |hovered: Mutable<bool>| menu_item("item 3", Checkbox::new(misc_demo_settings().checkbox.clone()).controlling_signal(hovered.signal()), hovered),
        |hovered: Mutable<bool>| menu_item("item 4", IterableOptions::new(options(4), misc_demo_settings().iterable_options.clone()), hovered),
        |hovered: Mutable<bool>| menu_item("master volume", Slider::new(audio_settings().master_volume.clone()).controlling_signal(hovered.signal()), hovered),
        |hovered: Mutable<bool>| menu_item("effect volume", Slider::new(audio_settings().effect_volume.clone()).controlling_signal(hovered.signal()), hovered),
        |hovered: Mutable<bool>| menu_item("music volume", Slider::new(audio_settings().music_volume.clone()).controlling_signal(hovered.signal()), hovered),
        |hovered: Mutable<bool>| menu_item("voice volume", Slider::new(audio_settings().voice_volume.clone()).controlling_signal(hovered.signal()), hovered),
    ];
    let l = item_funcs.len();
    let hovereds = MutableVec::new_with_values((0..l).into_iter().map(|_| Mutable::new(false)).collect());
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "audio menu")
    .update_raw_el(|raw_el|
        raw_el
        .on_signal_sync(
            {
                hovereds.signal_vec_cloned()
                .map_signal(|hovered| hovered.signal())
                .to_signal_map(|is_hovereds| !is_hovereds.iter().copied().any(identity))
            },
            |entity, none_focused| if none_focused { entity_focused_option().set(Some(entity)) }
        )
    )
    .apply(|element| {
        input_event_listener_controller(
            element,
            always(true),
            clone!((hovereds) move || {
                On::<MenuInputEvent>::run(clone!((hovereds) move |event: ListenerMut<MenuInputEvent>| {
                    let hovereds_lock = hovereds.lock_ref();
                    match event.input {
                        MenuInput::Up | MenuInput::Down => {
                            if let Some(i) = hovereds_lock.iter().position(|hovered| hovered.get()) {
                                hovereds_lock[i].set(false);
                                let new_i = if matches!(event.input, MenuInput::Up) { i + l - 1 } else { i + 1 } % l;
                                hovereds_lock[new_i].set(true);
                            } else {
                                hovereds_lock[if matches!(event.input, MenuInput::Up) { hovereds_lock.len() - 1 } else { 0 }].set(true);
                            }
                        },
                        MenuInput::Back => {
                            if hovereds_lock.iter().any(|hovered| hovered.get()) {
                                for hovered in hovereds_lock.iter() {
                                    hovered.set(false)
                                }
                            } else {
                                show_sub_menu().set(None);
                            }
                        },
                        _ => ()
                    }
                }))
            })
        )
    })
    .items({
        let hovereds = hovereds.lock_ref().into_iter().cloned().collect::<Vec<_>>();
        item_funcs.into_iter().zip(hovereds).enumerate()
        .map(move |(i, (f, hovered))|
            f(hovered).z_index(ZIndex::Local((l - i) as i32))
        )
    })
}

fn graphics_menu() -> Column<NodeBundle> {
    let preset_quality = graphics_settings().preset_quality.clone();
    let texture_quality = graphics_settings().texture_quality.clone();
    let shadow_quality = graphics_settings().shadow_quality.clone();
    let bloom_quality = graphics_settings().bloom_quality.clone();
    let non_preset_qualities = MutableVec::new_with_values(vec![texture_quality.clone(), shadow_quality.clone(), bloom_quality.clone()]);
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
            let mut qualities = qualities.into_iter();
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
        menu_item(
            "preset quality",
            Dropdown::new(MutableVec::new_with_values(Quality::iter().collect()), preset_quality, true),
            Mutable::new(false),
        ),
        menu_item(
            "texture quality",
            Dropdown::new(MutableVec::new_with_values(Quality::iter().collect()), texture_quality, false),
            Mutable::new(false),
        ),
        menu_item(
            "shadow quality",
            Dropdown::new(MutableVec::new_with_values(Quality::iter().collect()), shadow_quality, false),
            Mutable::new(false),
        ),
        menu_item(
            "bloom quality",
            Dropdown::new(MutableVec::new_with_values(Quality::iter().collect()), bloom_quality, false),
            Mutable::new(false),
        ),
    ];
    let l = items.len();
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "graphics menu")
    .update_raw_el(|raw_el| raw_el.hold_tasks([preset_broadcaster, preset_controller]))
    .items(
        items.into_iter().enumerate()
        .map(move |(i, item)| item.z_index(ZIndex::Local((l - i) as i32)))
    )
    .item(
        // solely here to dehover dropdown menu items  // TODO: this can also be solved by allowing setting Over/Out order at runtime or implementing .on_hovered_outside, i should do both of these
        El::<NodeBundle>::new()
        .with_style(move |style| style.height = Val::Px(SUB_MENU_HEIGHT - (l + 1) as f32 * MENU_ITEM_HEIGHT - BASE_PADDING * 2.))
        .on_hovered_change(|is_hovered| {
            if is_hovered {
                if let Some(hovered) = menu_item_hovered_option().take() {
                    hovered.set(false);
                }
            }
        })
    )
}

fn x_button(mut on_click: impl FnMut() + 'static + Send + Sync) -> impl Element {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
    .background_color(BackgroundColor(Color::NONE))
    .hovered_sync(hovered.clone())
    .update_raw_el(move |raw_el| {
        raw_el.insert(On::<Pointer<Click>>::run(move |mut event: ListenerMut<Pointer<Click>>| {
            // otherwise, e.g. clearing the dropdown will drop down the options too; the x should eat the click
            event.stop_propagation();
            on_click();
        }))
    })
    .child(
        El::<TextBundle>::new()
        .text(text("x"))
        .on_signal_with_text(
            hovered.signal().map_bool(|| Color::RED, || TEXT_COLOR),
            |text, color| {
                if let Some(section) = text.sections.first_mut() {
                    section.style.color = color;
                }
            },
        )
        // or like this:
        // .text_signal(
        //     hovered.signal().map_bool(|| Color::RED, || TEXT_COLOR)
        //     .map(|color| {
        //         Text::from_section(
        //             "x",
        //             TextStyle { font_size: 30.0, color, ..default() }
        //         )
        //     })
        // )
    )
}

#[static_ref]
fn sub_menu_selected() -> &'static Mutable<Option<SubMenu>> {
    Mutable::new(None)
}

#[static_ref]
fn entity_focused_option() -> &'static Mutable<Option<Entity>> {
    Mutable::new(None)
}

fn input_event_listener_controller<E: Element>(element: E, listening: impl Signal<Item = bool> + Send + 'static, mut callback: impl FnMut() -> On::<MenuInputEvent> + Send + 'static) -> E {
    element
    .update_raw_el(|raw_el| {
        raw_el
        .on_signal_with_entity(
            listening,
            move |entity, listening| {
                if listening {
                    entity.insert(callback());
                } else {
                    entity.remove::<On<MenuInputEvent>>();
                }
            }
        )
    })
}

#[static_ref]
fn show_sub_menu() -> &'static Mutable<Option<SubMenu>> {
    Mutable::new(None)
}

fn menu() -> impl Element {
    Stack::<NodeBundle>::new()
    .layer(
        menu_base(MAIN_MENU_SIDES, MAIN_MENU_SIDES, "main menu")
        .update_raw_el(|raw_el| {
            raw_el
            .on_signal_sync(show_sub_menu().signal(), |entity, sub_menu_option| {
                if sub_menu_option.is_none() {
                    entity_focused_option().set(Some(entity))
                }
            })
        })
        .apply(move |element| {
            input_event_listener_controller(
                element,
                show_sub_menu().signal_ref(Option::is_none),
                move || {
                    On::<MenuInputEvent>::run(move |event: ListenerMut<MenuInputEvent>| {
                        match event.input {
                            MenuInput::Up | MenuInput::Down => {
                                if let Some(cur_sub_menu) = sub_menu_selected().get() {
                                    if let Some(i) = SubMenu::iter().position(|sub_menu| cur_sub_menu == sub_menu) {
                                        let sub_menus = SubMenu::iter().collect::<Vec<_>>();
                                        sub_menu_selected().set(
                                            if matches!(event.input, MenuInput::Down) {
                                                sub_menus.iter().rev().cycle().skip(sub_menus.len() - i).next().copied()
                                            } else {
                                                sub_menus.iter().cycle().skip(i + 1).next().copied()
                                            }
                                        )
                                    }
                                } else {
                                    sub_menu_selected().set_neq(Some(if matches!(event.input, MenuInput::Up) { SubMenu::iter().last().unwrap() } else { SubMenu::iter().next().unwrap() }));
                                }
                            },
                            MenuInput::Select => {
                                if let Some(sub_menu) = sub_menu_selected().get() {
                                    show_sub_menu().set_neq(Some(sub_menu));
                                }
                            },
                            MenuInput::Back => {
                                sub_menu_selected().take();
                            },
                            _ =>()
                        }
                    })
                }
            )
        })
        .with_style(|style| style.row_gap = Val::Px(BASE_PADDING * 2.))
        .item(
            Column::<NodeBundle>::new()
            .with_style(|style| style.row_gap = Val::Px(BASE_PADDING))
            .align_content(Align::center())
            .items([
                sub_menu_button(SubMenu::Audio).hovered_signal(sub_menu_selected().signal_ref(|selected_option| selected_option == &Some(SubMenu::Audio))),
                sub_menu_button(SubMenu::Graphics).hovered_signal(sub_menu_selected().signal_ref(|selected_option| selected_option == &Some(SubMenu::Graphics))),
            ])
        )
    )
    .layer_signal(
        show_sub_menu().signal()
        .map_some(
            move |sub_menu| {
                let menu = match sub_menu {
                    SubMenu::Audio => audio_menu(),
                    SubMenu::Graphics => graphics_menu(),
                };
                Stack::<NodeBundle>::new()
                .with_style(|style| {
                    style.width =  Val::Px(SUB_MENU_WIDTH);
                    style.height =  Val::Px(SUB_MENU_HEIGHT);
                    // TODO: without absolute there's some weird bouncing when switching between menus, perhaps due to the layout system having to figure stuff out ?
                    style.position_type =  PositionType::Absolute;
                })
                .align(Align::center())
                .layer(menu.align(Align::center()))
                .layer(
                    x_button(clone!((show_sub_menu) move || { show_sub_menu().take(); }))
                    .align(Align::new().top().right())
                    .update_raw_el(|raw_el| {
                        raw_el.with_component::<Style>(|style| {
                            style.padding.right = Val::Px(BASE_PADDING);
                            style.padding.top = Val::Px(BASE_PADDING / 2.);
                        })
                    })
                )
            }
        )
    )
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Resource, Clone)]
struct AudioSettings {
    master_volume: Mutable<f32>,
    effect_volume: Mutable<f32>,
    music_volume: Mutable<f32>,
    voice_volume: Mutable<f32>,
}

#[static_ref]
fn audio_settings() -> &'static AudioSettings {
    AudioSettings {
        master_volume: Mutable::new(100.),
        effect_volume: Mutable::new(50.),
        music_volume: Mutable::new(50.),
        voice_volume: Mutable::new(50.),
    }
}

#[derive(Resource, Clone)]
struct GraphicsSettings {
    preset_quality: Mutable<Option<Quality>>,
    texture_quality: Mutable<Option<Quality>>,
    shadow_quality: Mutable<Option<Quality>>,
    bloom_quality: Mutable<Option<Quality>>,
}

#[static_ref]
fn graphics_settings() -> &'static GraphicsSettings {
    GraphicsSettings {
        preset_quality: Mutable::new(Some(Quality::Medium)),
        texture_quality: Mutable::new(Some(Quality::Medium)),
        shadow_quality: Mutable::new(Some(Quality::Medium)),
        bloom_quality: Mutable::new(Some(Quality::Medium)),
    }
}

#[derive(Resource, Clone)]
struct MiscDemoSettings {
    dropdown: Mutable<Option<String>>,
    mutually_exclusive_options: Mutable<Option<usize>>,
    checkbox: Mutable<bool>,
    iterable_options: Mutable<String>,
}

#[static_ref]
fn misc_demo_settings() -> &'static MiscDemoSettings {
    MiscDemoSettings {
        dropdown: Mutable::new(None),
        mutually_exclusive_options: Mutable::new(None),
        checkbox: Mutable::new(false),
        iterable_options: Mutable::new("option 1".to_string()),
    }
}

#[derive(Clone, Copy)]
enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

#[derive(Clone, Event, EntityEvent)]
struct MenuInputEvent {
    #[target]
    entity: Entity,
    input: MenuInput,
}

fn keyboard_events(
    keys: Res<Input<KeyCode>>,
    mut menu_input_events: EventWriter<MenuInputEvent>,
) {
    if let Some(entity) = entity_focused_option().get() {
        for (key_code, input) in [
            (KeyCode::Up, MenuInput::Up),
            (KeyCode::Down, MenuInput::Down),
            (KeyCode::Left, MenuInput::Left),
            (KeyCode::Right, MenuInput::Right),
            (KeyCode::Return, MenuInput::Select),
            (KeyCode::Escape, MenuInput::Back),
        ] {
            if keys.just_pressed(key_code) {
                menu_input_events.send(MenuInputEvent { entity, input });
            }
        }
    }
}

fn spawn_ui_root(world: &mut World) {
    world.insert_resource(audio_settings().clone());
    world.insert_resource(graphics_settings().clone());
    world.insert_resource(misc_demo_settings().clone());
    El::<NodeBundle>::new()
    .with_style(|style| {
        style.width = Val::Percent(100.0);
        style.height = Val::Percent(100.0);
    })
    .align_content(Align::center())
    .child(menu())
    .spawn(world);
}
