use super::{
    pointer_event_aware::PointerEventAware,
    raw::{AppendDirection, RawElWrapper, RawHaalkaEl},
    utils::{clone, spawn},
};
use bevy::{
    ecs::component::Component,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use futures_signals::signal::{always, BoxSignal, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::{SignalExtBool, SignalExtExt};
use std::convert::Into;

/// Enables an element's viewport to be modified and react to mouse wheel events.
pub trait Scrollable: RawElWrapper {
    /// Wrap this element in a scrollable container, setting how mouse wheel events should be
    /// handled via [`ScrollabilitySettings`], and activating this handling only when the
    /// provided [`Signal`] outputs `true`.
    fn scrollable(
        self,
        settings: ScrollabilitySettings,
        active_signal: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(move |raw_el| {
            raw_el
                .insert(ScrollHandler(settings.scroll_handler))
                .component_signal::<ScrollableMarker, _>(active_signal.map_true(|| ScrollableMarker))
                .defer_update(AppendDirection::Front, move |raw_el| {
                    RawHaalkaEl::from(NodeBundle::default())
                        .with_component::<Style>(move |mut style| {
                            style.flex_direction = settings.flex_direction;
                            style.overflow = settings.overflow;
                        })
                        .child(raw_el)
                })
        })
    }
}

/// Convenience trait for enabling scrollability when hovering over an element.
pub trait HoverableScrollable: Scrollable + PointerEventAware {
    /// Wrap this element in a scrollable container, setting how mouse wheel events should be
    /// handled via [`ScrollabilitySettings`], and activating this handling only when this
    /// element is hovered.
    fn scrollable_on_hover(self, settings: ScrollabilitySettings) -> Self {
        let hovered = Mutable::new(false);
        self.scrollable(settings, hovered.signal()).hovered_sync(hovered)
    }
}

impl<T: Scrollable + PointerEventAware> HoverableScrollable for T {}

#[derive(Component)]
struct ScrollHandler(Box<dyn FnMut(&MouseWheel, &Node, Mut<Style>, &Parent, &Query<&Node>) + Send + Sync + 'static>);

/// Configuration for scrollable wrapping container and handling of mouse wheel events.
pub struct ScrollabilitySettings {
    /// Forwarded directly to the wrapping container's [`Style`].
    pub flex_direction: FlexDirection,
    /// Forwarded directly to the wrapping container's [`Style`].
    pub overflow: Overflow,
    /// Function to handle mouse wheel events, with access to the element's [`Node`],
    /// [`Mut<Style>`], [`Parent`], and a [`Query<&Node>`].
    pub scroll_handler: Box<dyn FnMut(&MouseWheel, &Node, Mut<Style>, &Parent, &Query<&Node>) + Send + Sync + 'static>,
}

#[derive(Component)]
pub struct ScrollableMarker;

fn scroll_system(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut scroll_targets: Query<(&mut ScrollHandler, &Node, &mut Style, &Parent), With<ScrollableMarker>>,
    node_query: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (mut scroll_handler, scrollable_node, style, parent) in &mut scroll_targets {
            (scroll_handler.0)(mouse_wheel_event, scrollable_node, style, parent, &node_query);
        }
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub enum ScrollDirection {
    Horizontal,
    Vertical,
}

/// Allows setting the direction and magnitude (in pixels) of viewport movement in response to mouse
/// wheel events. These settings can be either static or reactive via [`Signal`]s.
pub struct BasicScrollHandler {
    direction: Option<BoxSignal<'static, ScrollDirection>>,
    magnitude: Option<BoxSignal<'static, f32>>,
}

impl BasicScrollHandler {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self {
            direction: None,
            magnitude: None,
        }
    }

    /// Reactively set the [`ScrollDirection`] of viewport movement in response to mouse wheel
    /// events.
    pub fn direction_signal<S: Signal<Item = ScrollDirection> + Send + 'static>(
        mut self,
        direction_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(direction_signal) = direction_signal_option.into() {
            self.direction = Some(direction_signal.boxed());
        }
        self
    }

    /// Set the [`ScrollDirection`] of viewport movement in response to mouse wheel events.
    pub fn direction(mut self, direction_option: impl Into<Option<ScrollDirection>>) -> Self {
        if let Some(direction) = direction_option.into() {
            self = self.direction_signal(always(direction));
        }
        self
    }

    /// Reactively set the magnitude (in pixels) of viewport movement in response to mouse wheel
    /// events.
    pub fn pixels_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        pixels_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(pixels_signal) = pixels_signal_option.into() {
            self.magnitude = Some(pixels_signal.boxed());
        }
        self
    }

    /// Set the magnitude (in pixels) of viewport movement in response to mouse wheel events.
    pub fn pixels(mut self, pixels_option: impl Into<Option<f32>>) -> Self {
        if let Some(pixels) = pixels_option.into() {
            self = self.pixels_signal(always(pixels));
        }
        self
    }
}

const DEFAULT_SCROLL_DIRECTION: ScrollDirection = ScrollDirection::Vertical;
const DEFAULT_SCROLL_MAGNITUDE: f32 = 10.;

impl From<BasicScrollHandler>
    for Box<dyn FnMut(&MouseWheel, &Node, Mut<Style>, &Parent, &Query<&Node>) + Send + Sync + 'static>
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
        let f = {
            move |mouse_wheel_event: &MouseWheel,
                  scrollable_node: &Node,
                  mut style: Mut<Style>,
                  parent: &Parent,
                  node_query: &Query<&Node>| {
                match direction.get() {
                    ScrollDirection::Vertical => {
                        let height = scrollable_node.size().y;
                        let Ok(container_height) = node_query.get(parent.get()).map(|node| node.size().y) else {
                            return;
                        };
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
                        let Ok(container_width) = node_query.get(parent.get()).map(|node| node.size().x) else {
                            return;
                        };
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
            }
        };
        Box::new(f)
    }
}

pub(crate) struct ScrollablePlugin;

impl Plugin for ScrollablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, scroll_system.run_if(any_with_component::<ScrollableMarker>));
    }
}
