use super::{
    pointer_event_aware::PointerEventAware,
    raw::{observe, register_system, utils::remove_system_holder_on_remove},
    utils::{clone, spawn},
    viewport_mutable::{ViewportMutable, ViewportMutation},
};
use apply::Apply;
use bevy::{
    ecs::component::Component,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use futures_signals::signal::{always, BoxSignal, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::{SignalExtBool, SignalExtExt};
use std::convert::Into;

#[derive(Component, Default)]
pub struct ScrollDisabled;

#[derive(Component)]
struct ScrollEnabled;

/// Enables an element's viewport to be modified and react to mouse wheel events.
pub trait MouseWheelScrollable: ViewportMutable {
    fn on_scroll_with_system_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<(Entity, MouseWheel), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .insert(ScrollEnabled)
                .observe(|event: Trigger<OnAdd, Disabled>, mut commands: Commands| {
                    if let Some(mut entity) = commands.get_entity(event.entity()) {
                        entity.remove::<ScrollEnabled>();
                    }
                })
                .observe(move |event: Trigger<OnRemove, Disabled>, mut commands: Commands| {
                    if let Some(mut entity) = commands.get_entity(event.entity()) {
                        entity.try_insert(ScrollEnabled);
                    }
                })
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    system_holder.set(Some(system));
                    observe(world, entity, move |mouse_wheel: Trigger<MouseWheel>, mut commands: Commands| {
                        commands.run_system_with_input(system, (mouse_wheel.entity(), *mouse_wheel.event()));
                    })
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    fn on_scroll_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, MouseWheel), (), Marker> + Send + 'static,
    ) -> Self {
        self.on_scroll_with_system_disableable::<ScrollDisabled, Marker>(handler)
    }

    fn on_scroll_with_system_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<(Entity, MouseWheel), (), Marker> + Send + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.component_signal::<ScrollDisabled, _>(blocked.map_true(default)))
            .on_scroll_with_system_disableable::<ScrollDisabled, _>(handler)
    }

    fn on_scroll_disableable<Disabled: Component>(
        self,
        mut handler: impl FnMut(MouseWheel) + Send + Sync + 'static,
    ) -> Self {
        self.on_scroll_with_system_disableable::<Disabled, _>(move |In((_, mouse_wheel))| handler(mouse_wheel))
    }

    fn on_scroll(self, handler: impl FnMut(MouseWheel) + Send + Sync + 'static) -> Self {
        self.on_scroll_disableable::<ScrollDisabled>(handler)
    }

    fn on_scroll_disableable_signal(
        self,
        handler: impl FnMut(MouseWheel) + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.component_signal::<ScrollDisabled, _>(blocked.map_true(default)))
            .on_scroll_disableable::<ScrollDisabled>(handler)
    }
}

/// Convenience trait for enabling scrollability when hovering over an element.
pub trait OnHoverMouseWheelScrollable: MouseWheelScrollable + PointerEventAware {
    fn on_scroll_with_system_on_hover<Marker>(
        self,
        handler: impl IntoSystem<(Entity, MouseWheel), (), Marker> + Send + 'static,
    ) -> Self {
        self.on_scroll_with_system_disableable::<ScrollDisabled, Marker>(handler)
            .on_hovered_change_with_system(|In((entity, hovered)), mut commands: Commands| {
                if let Some(mut entity) = commands.get_entity(entity) {
                    if hovered {
                        entity.remove::<ScrollDisabled>();
                    } else {
                        entity.try_insert(ScrollDisabled);
                    }
                }
            })
    }

    fn on_scroll_on_hover(self, mut handler: impl FnMut(MouseWheel) + Send + Sync + 'static) -> Self {
        self.on_scroll_with_system_on_hover::<_>(move |In((_, mouse_wheel))| handler(mouse_wheel))
    }
}

impl<T: PointerEventAware + MouseWheelScrollable> OnHoverMouseWheelScrollable for T {}

fn scroll_system(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    scroll_listeners: Query<Entity, With<ScrollEnabled>>,
    mut commands: Commands,
) {
    let listeners = scroll_listeners.iter().collect::<Vec<_>>();
    for &event in mouse_wheel_events.read() {
        // TODO: after 0.15 use &listeners
        commands.trigger_targets(event, listeners.clone());
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub enum ScrollDirection {
    Horizontal,
    Vertical,
    Both,
}

/// Allows setting the direction and magnitude (in pixels) of viewport movement in response to mouse
/// wheel events. These settings can be either static or reactive via [`Signal`]s.
pub struct BasicScrollHandler {
    direction: Option<BoxSignal<'static, ScrollDirection>>,
    magnitude: Option<BoxSignal<'static, f32>>,
}

const DEFAULT_SCROLL_DIRECTION: ScrollDirection = ScrollDirection::Vertical;
const DEFAULT_SCROLL_MAGNITUDE: f32 = 10.;

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

    // TODO: is there a better return type for this ?
    pub fn into_system(
        self,
    ) -> Box<
        dyn FnMut(In<(Entity, MouseWheel)>, Query<&Style>, Res<ButtonInput<KeyCode>>, Commands) + Send + Sync + 'static,
    > {
        let BasicScrollHandler {
            direction: direction_signal_option,
            magnitude: magnitude_signal_option,
        } = self;
        let direction = Mutable::new(DEFAULT_SCROLL_DIRECTION);
        let magnitude = Mutable::new(DEFAULT_SCROLL_MAGNITUDE);
        if let Some(direction_signal) = direction_signal_option {
            // TODO: these "leak" for as long as the source mutable is alive, is this an issue? revert to less
            // ergonomic task collection strat if so
            direction_signal
                .for_each_sync(clone!((direction) move |d| direction.set_neq(d)))
                .apply(spawn)
                .detach()
        }
        if let Some(magnitude_signal) = magnitude_signal_option {
            // TODO: these "leak" for as long as the source mutable is alive, is this an issue? revert to less
            // ergonomic task collection strat if so
            magnitude_signal
                .for_each_sync(clone!((magnitude) move |m| magnitude.set_neq(m)))
                .apply(spawn)
                .detach()
        }
        let f = move |In((entity, mouse_wheel)): In<(Entity, MouseWheel)>,
                      styles: Query<&Style>,
                      keys: Res<ButtonInput<KeyCode>>,
                      mut commands: Commands| {
            let dy = if mouse_wheel.y.is_sign_negative() { -1. } else { 1. } * magnitude.get();
            if let Ok(style) = styles.get(entity) {
                let direction = direction.get();
                if matches!(direction, ScrollDirection::Vertical)
                    || matches!(direction, ScrollDirection::Both)
                        && !(keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
                {
                    let top = match style.top {
                        Val::Px(top) => top,
                        _ => 0.,
                    };
                    commands.trigger_targets(ViewportMutation::y(top + dy), entity);
                } else if matches!(direction, ScrollDirection::Horizontal)
                    || matches!(direction, ScrollDirection::Both)
                        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
                {
                    let left = match style.left {
                        Val::Px(left) => left,
                        _ => 0.,
                    };
                    commands.trigger_targets(ViewportMutation::x(left + dy), entity);
                }
            }
        };
        Box::new(f)
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, scroll_system.run_if(any_with_component::<ScrollEnabled>));
}
