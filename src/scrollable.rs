use crate::{clone, raw_el::AppendDirection, spawn, PointerEventAware, RawElWrapper, RawHaalkaEl};
use bevy::{
    ecs::component::Component,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use futures_signals::signal::{always, BoxSignal, Mutable, Signal, SignalExt};
use futures_signals_ext::{SignalExtBool, SignalExtExt};

pub trait Scrollable: RawElWrapper {
    fn scrollable(
        self,
        settings: ScrollabilitySettings,
        active_signal: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(move |raw_el| {
            raw_el.defer_update(AppendDirection::Front, move |raw_el| {
                RawHaalkaEl::from(NodeBundle::default())
                    .with_component::<Style>(move |style| {
                        style.flex_direction = settings.flex_direction;
                        style.overflow = settings.overflow;
                    })
                    .child(
                        raw_el
                            .insert(ScrollHandler(settings.scroll_handler))
                            .component_signal::<ScrollableMarker>(active_signal.map_true(|| ScrollableMarker)),
                    )
            })
        })
    }
}

pub trait HoverableScrollable: Scrollable + PointerEventAware {
    fn scrollable_on_hover(self, settings: ScrollabilitySettings) -> Self {
        let hovered = Mutable::new(false);
        self.scrollable(settings, hovered.signal()).hovered_sync(hovered)
    }
}

impl<T: Scrollable + PointerEventAware> HoverableScrollable for T {}

#[derive(Component)]
struct ScrollHandler(Box<dyn FnMut(&MouseWheel, &mut Style, &Parent, &Node, &Query<&Node>) + Send + Sync + 'static>);

pub struct ScrollabilitySettings {
    pub flex_direction: FlexDirection,
    pub overflow: Overflow,
    pub scroll_handler: Box<dyn FnMut(&MouseWheel, &mut Style, &Parent, &Node, &Query<&Node>) + Send + Sync + 'static>,
}

#[derive(Component)]
pub struct ScrollableMarker;

fn scroll_system(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut scroll_targets: Query<(&mut ScrollHandler, &mut Style, &Parent, &Node), With<ScrollableMarker>>,
    node_query: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (mut scroll_handler, mut style, parent, scrollable_node) in &mut scroll_targets {
            (scroll_handler.0)(mouse_wheel_event, &mut style, parent, scrollable_node, &node_query);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ScrollDirection {
    Horizontal,
    Vertical,
}

pub struct BasicScrollHandler {
    direction: Option<BoxSignal<'static, ScrollDirection>>,
    magnitude: Option<BoxSignal<'static, f32>>,
}

impl BasicScrollHandler {
    pub fn new() -> Self {
        Self {
            direction: None,
            magnitude: None,
        }
    }

    pub fn direction_signal(mut self, direction_signal: impl Signal<Item = ScrollDirection> + Send + 'static) -> Self {
        self.direction = Some(direction_signal.boxed());
        self
    }

    pub fn direction(self, direction: ScrollDirection) -> Self {
        self.direction_signal(always(direction))
    }

    pub fn pixels_signal(mut self, pixels_signal: impl Signal<Item = f32> + Send + 'static) -> Self {
        self.magnitude = Some(pixels_signal.boxed());
        self
    }

    pub fn pixels(self, pixels: f32) -> Self {
        self.pixels_signal(always(pixels))
    }
}

const DEFAULT_SCROLL_DIRECTION: ScrollDirection = ScrollDirection::Vertical;
const DEFAULT_SCROLL_MAGNITUDE: f32 = 10.;

impl From<BasicScrollHandler>
    for Box<dyn FnMut(&MouseWheel, &mut Style, &Parent, &Node, &Query<&Node>) + Send + Sync + 'static>
{
    fn from(handler: BasicScrollHandler) -> Self {
        let BasicScrollHandler {
            direction: direction_signal_option,
            magnitude: magnitude_signal_option,
        } = handler;
        let direction = Mutable::new(DEFAULT_SCROLL_DIRECTION);
        let magnitude = Mutable::new(DEFAULT_SCROLL_MAGNITUDE);
        if let Some(direction_signal) = direction_signal_option {
            // TODO: these "leak" for as long as the source mutable is alive, is this an issue? revert to less
            // ergonomic task collection strat if so
            spawn(direction_signal.for_each_sync(clone!((direction) move |d| direction.set_neq(d)))).detach();
        }
        if let Some(magnitude_signal) = magnitude_signal_option {
            // TODO: these "leak" for as long as the source mutable is alive, is this an issue? revert to less
            // ergonomic task collection strat if so
            spawn(magnitude_signal.for_each_sync(clone!((magnitude) move |m| magnitude.set_neq(m)))).detach();
        }
        Box::new(
            move |mouse_wheel_event: &MouseWheel,
                  style: &mut Style,
                  parent: &Parent,
                  scrollable_node: &Node,
                  node_query: &Query<&Node>| {
                match direction.get() {
                    ScrollDirection::Vertical => {
                        let height = scrollable_node.size().y;
                        let container_height = node_query.get(parent.get()).unwrap().size().y;
                        let max_scroll: f32 = (height - container_height).max(0.);
                        let dy = match mouse_wheel_event.unit {
                            MouseScrollUnit::Line => magnitude.get() * mouse_wheel_event.y,
                            MouseScrollUnit::Pixel => mouse_wheel_event.y,
                        };
                        if let Val::Auto = style.top {
                            style.top = Val::Px(0.);
                        }
                        if let Val::Px(cur) = style.top {
                            style.top = Val::Px((cur + dy).clamp(-max_scroll, 0.));
                        }
                    }
                    ScrollDirection::Horizontal => {
                        let width = scrollable_node.size().x;
                        let container_width = node_query.get(parent.get()).unwrap().size().x;
                        let max_scroll: f32 = (width - container_width).max(0.);
                        let dx = match mouse_wheel_event.unit {
                            MouseScrollUnit::Line => mouse_wheel_event.y * magnitude.get(),
                            MouseScrollUnit::Pixel => mouse_wheel_event.y,
                        };
                        if let Val::Auto = style.left {
                            style.left = Val::Px(0.);
                        }
                        if let Val::Px(cur) = style.left {
                            style.left = Val::Px((cur + dx).clamp(-max_scroll, 0.));
                        }
                    }
                }
            },
        )
    }
}

pub(crate) struct ScrollablePlugin;

impl Plugin for ScrollablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, scroll_system.run_if(any_with_component::<ScrollableMarker>));
    }
}
