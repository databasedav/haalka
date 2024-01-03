// Main menu with sub menus for audio and graphics.
// Simple buttons for option selection.
// Slider for volume.
// Dropdown for graphics quality (low/medium/high).
// Navigation possible with mouse, keyboard and controller.
//     Mouse: Separate styles for hover and press.
//     Keyboard/Controller: Separate styles for currently focused element.

use std::{fmt::Display, time::Duration};

use bevy::prelude::*;
use haalka::*;
use futures_signals::map_ref;
use futures_signals_ext::*;
use async_io::Timer;
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
        .add_systems(Startup, (setup, spawn_ui_root))
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const FONT_SIZE: f32 = 30.;
const MAIN_MENU_SIDES: f32 = 300.;
const SUB_MENU_HEIGHT: f32 = 700.;
const SUB_MENU_WIDTH: f32 = 1200.;
const BASE_PADDING: f32 = 10.;
const DEFAULT_BUTTON_HEIGHT: f32 = 65.;
const BASE_BORDER_WIDTH: f32 = 5.;
const MENU_ITEM_HEIGHT: f32 = DEFAULT_BUTTON_HEIGHT + BASE_PADDING;

#[derive(Clone, Copy, PartialEq, Display)]
enum SubMenu {
    Audio,
    Graphics,
}

struct Button {
    el: El<ButtonBundle>,
    pressed: Mutable<bool>,
    selected: Mutable<bool>,
}

impl RawElWrapper for Button {
    type NodeType = ButtonBundle;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<Self::NodeType> {
        self.el.raw_el_mut()
    }
}

impl Button {
    fn new() -> Self {
        let selected = Mutable::new(false);
        let hovered = Mutable::new(false);
        let pressed = Mutable::new(false);
        let pressed_or_selected = signal::or(selected.signal(), pressed.signal());
        let selected_hovered_broadcaster = map_ref! {
            let pressed_or_selected = pressed_or_selected,
            let hovered = hovered.signal() => {
                (*pressed_or_selected, *hovered)
            }
        }.broadcast();
        let border_color_signal = {
            selected_hovered_broadcaster.signal()
            .map(|(selected, hovered)| {
                if selected {
                    Color::RED
                } else if hovered {
                    Color::WHITE
                } else {
                    Color::BLACK
                }
            })
            .map(BorderColor)
        };
        let background_color_signal = {
            selected_hovered_broadcaster.signal()
            .map(|(selected, hovered)| {
                if selected {
                    PRESSED_BUTTON
                } else if hovered {
                    HOVERED_BUTTON
                } else {
                    NORMAL_BUTTON
                }
            })
            .map(BackgroundColor)
        };
        Self {
            el: {
                El::<ButtonBundle>::new()
                .with_style(move |style| {
                    style.height = Val::Px(DEFAULT_BUTTON_HEIGHT);
                    style.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                })
                .align_content(Align::center())
                .on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
                .border_color_signal(border_color_signal)
                .background_color_signal(background_color_signal)
            },
            pressed,
            selected,
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

    fn body_signal(mut self, body_option_signal: impl Signal<Item = impl IntoOptionElement + 'static> + Send + 'static) -> Self {
        self.el = self.el.child_signal(body_option_signal);
        self
    }

    fn on_press(mut self, mut on_press: impl FnMut() + 'static + Send + Sync) -> Self {
        self.el = {
            self.el
            .on_pressed_change(clone!((self.pressed => pressed) move |is_pressed| {
                if is_pressed { on_press() }
                pressed.set_neq(is_pressed);
            }))
        };
        self
    }

    fn selected_signal(mut self, selected_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let task = spawn(sync(self.selected.clone(), selected_signal));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([task]));
        self
    }
}

async fn sync<T>(mutable: Mutable<T>, signal: impl Signal<Item = T> + Send + 'static) {
    signal.for_each_sync(move |value| {
        mutable.set(value);
    })
    .await;
}

fn text(text: &str) -> Text {
    Text::from_section(text, TextStyle { font_size: FONT_SIZE, ..default() })
}

fn text_button(text_signal: impl Signal<Item = String> + Send + 'static, on_press: impl FnMut() + 'static + Send + Sync) -> Button {
    Button::new()
    .width(Val::Px(200.))
    .body_signal(always(El::<TextBundle>::new().text_signal(text_signal.map(|t| text(&t)))))
    .on_press(on_press)
}

fn sub_menu_button(sub_menu: SubMenu, show_sub_menu: Mutable<Option<SubMenu>>) -> impl Element {
    text_button(
        always(sub_menu.to_string()),
        clone!((show_sub_menu) move || { show_sub_menu.set_neq(Some(sub_menu)) })
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

fn dropdown<T: Clone + PartialEq + Display + Send + Sync + 'static>(options: MutableVec<T>, selected: Mutable<Option<T>>, clearable: bool) -> El<NodeBundle> {
    let show_dropdown = Mutable::new(false);
    El::<NodeBundle>::new()
    .child(
        Button::new()
        .width(Val::Px(300.))
        .body_signal(always(
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
                        selected.signal_ref(Option::is_some).map_true(clone!((selected) move || x_button(clone!((selected) move || { selected.take(); })))).boxed()
                    } else {
                        always(None).boxed()
                    }
                })
                .item(
                    El::<TextBundle>::new()
                    // TODO: need to figure out to rotate in place (around center)
                    // .on_signal_with_transform(show_dropdown.signal(), |transform, showing| {
                    //     transform.rotation = Quat::from_rotation_z((if showing { 180.0f32 } else { 0. }).to_radians());
                    // })
                    .text(text("v"))
                )
            )
        ))
        .on_press(clone!((show_dropdown) move || { flip(&show_dropdown) }))
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
                .filter_signal_cloned(clone!((selected) move |option| {
                    selected.signal_ref(clone!((option) move |selected_option| {
                        selected_option.as_ref() != Some(&option)
                    }))
                    .dedupe()
                }))
                .map(clone!((selected, show_dropdown) move |option| {
                    text_button(
                        always(option.to_string()),
                        clone!((selected, show_dropdown, option) move || {
                            selected.set_neq(Some(option.clone()));
                            flip(&show_dropdown);
                        })
                    )
                    .width(Val::Percent(100.))
                }))
            )
        }))
    )
}

fn checkbox(checked: Mutable<bool>) -> Button {
    Button::new()
    .width(Val::Px(30.))
    .height(Val::Px(30.))
    .on_press(clone!((checked) move || { flip(&checked) }))
    .selected_signal(checked.signal())
}

#[derive(Clone, Copy, EnumIter, PartialEq, Display)]
enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

fn signal_eq<T: PartialEq + Send>(signal1: impl Signal<Item = T> + Send + 'static, signal2: impl Signal<Item = T> + Send + 'static) -> impl Signal<Item = bool> + Send + 'static {
    map_ref! {
        let signal1 = signal1,
        let signal2 = signal2 => {
            signal1 == signal2
        }
    }
    .dedupe()
}

fn mutually_exclusive_options<T: Clone + PartialEq + Display + Send + Sync + 'static>(options: MutableVec<T>) -> impl Element + Alignable {
    let selected = Mutable::new(None);
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
}

fn options(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("option {}", i)).collect()
}

fn menu_item(label: &str, body: impl Element + Alignable) -> Stack<ButtonBundle> {
    let hovered = Mutable::new(false);
    Stack::<ButtonBundle>::new()
    // TODO: need to migrate interactions to mod picking to capture child hovering
    .on_hovered_change(clone!((hovered) move |is_hovered| hovered.set_neq(is_hovered)))
    .background_color_signal(
        hovered.signal()
        .map_bool(
            || NORMAL_BUTTON.with_l(NORMAL_BUTTON.l() + 0.1),
            || NORMAL_BUTTON,
        )
        .map(BackgroundColor)
    )
    .with_style(|style| {
        style.width = Val::Percent(100.);
        style.padding = UiRect::axes(Val::Px(BASE_PADDING), Val::Px(BASE_PADDING / 2.));
        style.height = Val::Px(MENU_ITEM_HEIGHT);
    })
    .layer(El::<TextBundle>::new().text(text(label)).align(Align::new().left().center_y()))
    .layer(body.align(Align::new().right().center_y()))
}

fn audio_menu() -> Column<NodeBundle> {
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "audio menu")
    .item(
        menu_item(
            "item 1",
            dropdown(MutableVec::new_with_values(options(4)), Mutable::new(None), true),
        )
        .z_index(ZIndex::Local(1))
    )
    .item(
        menu_item(
            "item 2",
            mutually_exclusive_options(MutableVec::new_with_values(options(3))),
        )
    )
    .item(
        menu_item(
            "item 3",
            checkbox(Mutable::new(false)).el,
        )
    )
    // slider (migrate interations to mod picking)
    // iterate with left/right arrows
}

fn graphics_menu() -> Column<NodeBundle> {
    let preset_quality = Mutable::new(Some(Quality::Medium));
    let texture_quality = Mutable::new(Some(Quality::Medium));
    let shadow_quality = Mutable::new(Some(Quality::Medium));
    let bloom_quality = Mutable::new(Some(Quality::Medium));
    let preset_broadcaster = spawn(clone!((preset_quality, texture_quality, shadow_quality, bloom_quality) async move {
        preset_quality.signal()
        .for_each_sync(|quality_option| {
            if let Some(quality) = quality_option {
                texture_quality.set_neq(Some(quality));
                shadow_quality.set_neq(Some(quality));
                bloom_quality.set_neq(Some(quality));
            }
        })
        .await;
    }));
    let preset_unsetter = spawn(clone!((preset_quality, texture_quality, shadow_quality, bloom_quality) async move {
        map_ref! {
            let texture = texture_quality.signal(),
            let shadow = shadow_quality.signal(),
            let bloom = bloom_quality.signal() => {
                let mut preset = preset_quality.lock_mut();
                if preset.is_none() && texture == shadow && shadow == bloom {
                    *preset = *texture;
                }
                else if preset.is_some() && (&*preset != texture || &*preset != shadow || &*preset != bloom) {
                    *preset = None;
                }
            }
        }
        .to_future()
        .await;
    }));
    let items = [
        menu_item(
            "preset quality",
            dropdown(MutableVec::new_with_values(Quality::iter().collect()), preset_quality, true),
        ),
        menu_item(
            "texture quality",
            dropdown(MutableVec::new_with_values(Quality::iter().collect()), texture_quality, false)
        ),
        menu_item(
            "shadow quality",
            dropdown(MutableVec::new_with_values(Quality::iter().collect()), shadow_quality, false)
        ),
        menu_item(
            "bloom quality",
            dropdown(MutableVec::new_with_values(Quality::iter().collect()), bloom_quality, false)
        ),
    ];
    let l = items.len();
    menu_base(SUB_MENU_WIDTH, SUB_MENU_HEIGHT, "graphics menu")
    .update_raw_el(|raw_el| raw_el.hold_tasks([preset_broadcaster, preset_unsetter]))
    .items(
        items.into_iter().enumerate()
        .map(move |(i, item)| item.z_index(ZIndex::Local((l - i) as i32)))
    )
}

fn x_button(mut on_press: impl FnMut() + 'static + Send + Sync) -> impl Element + RawElWrapper + Alignable {
    let hovered = Mutable::new(false);
    El::<ButtonBundle>::new()
    .background_color(BackgroundColor(Color::NONE))
    .on_hovered_change(clone!((hovered) move |is_hovered| hovered.set_neq(is_hovered)))
    .on_pressed_change(move |is_pressed| if is_pressed { on_press() })
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
        //         Text::from_section("x", TextStyle {
        //             font_size: 30.0,
        //             color,
        //             ..default()
        //         })
        //     })
        // )
    )
}

fn menu() -> impl Element {
    let show_sub_menu = Mutable::new(Some(SubMenu::Audio));
    Stack::<NodeBundle>::new()
    .layer(
        menu_base(MAIN_MENU_SIDES, MAIN_MENU_SIDES, "main menu")
        .with_style(|style| style.row_gap = Val::Px(BASE_PADDING * 2.))
        .item(
            Column::<NodeBundle>::new()
            .with_style(|style| style.row_gap = Val::Px(BASE_PADDING))
            .align_content(Align::center())
            .items([
                sub_menu_button(SubMenu::Audio, show_sub_menu.clone()),
                sub_menu_button(SubMenu::Graphics, show_sub_menu.clone()),
            ])
        )
    )
    .layer_signal(
        show_sub_menu.signal()
        .map_some(
            move |sub_menu| {
                let menu = match sub_menu {
                    SubMenu::Audio => audio_menu(),
                    SubMenu::Graphics => graphics_menu(),
                };
                // let align = Mutable::new(Some(Align::new().top().right()));
                // let task = spawn(clone!((align) async move {
                //     loop {
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().right().center_y());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().bottom().right());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().bottom().center_x());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().bottom().left());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().left().center_y());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().top().left());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().top().center_x());
                //         Timer::after(Duration::from_millis(1000)).await;
                //         *align.lock_mut() = Some(Align::new().top().right());
                //     }
                // }));
                Stack::<NodeBundle>::new()
                // .update_raw_el(|raw_el| raw_el.hold_tasks([task]))
                .with_style(|style| {
                    style.width =  Val::Px(SUB_MENU_WIDTH);
                    style.height =  Val::Px(SUB_MENU_HEIGHT);
                    // TODO: without absolute there's some weird bouncing when switching between menus, perhaps due to the layout system having to figure stuff out ?
                    style.position_type =  PositionType::Absolute;
                })
                .align(Align::center())
                .layer(menu.align(Align::center()))
                .layer(
                    x_button(clone!((show_sub_menu) move || { show_sub_menu.take(); }))
                    .align(Align::new().top().right())
                    // .align_signal(align.signal_cloned())
                    .update_raw_el(|raw_el| {
                        raw_el.with_component::<Style>(|style| {
                            style.padding = UiRect::new(Val::Px(0.), Val::Px(BASE_PADDING), Val::Px(BASE_PADDING / 2.), Val::Px(0.));
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

fn spawn_ui_root(world: &mut World) {
    El::<NodeBundle>::new()
    .with_style(|style| {
        style.width = Val::Percent(100.0);
        style.height = Val::Percent(100.0);
    })
    .align_content(Align::center())
    .child(menu())
    .spawn(world);
}
