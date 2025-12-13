//! Semantics for managing [`ViewportMutable`] [`Element`](super::element::Element)s that react to
//! mouse wheel events.

use super::{
    pointer_event_aware::{HoverData, PointerEventAware},
    utils::{clone, observe, register_system, remove_system_holder_on_remove},
    viewport_mutable::ViewportMutable,
};
use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::{mouse::*, prelude::*};
use bevy_ui::prelude::*;
use jonmo::signal::{Signal, SignalExt};
use std::sync::{Arc, OnceLock};

/// Marker [`Component`] that disables an element's viewport from reacting to mouse wheel events.
#[derive(Component, Default, Clone)]
pub struct ScrollDisabled;

#[derive(Component)]
struct ScrollEnabled;

/// Enables an element's viewport to be modified and react to mouse wheel events.
pub trait MouseWheelScrollable: ViewportMutable {
    /// When this element receives a [`MouseWheel`] event, if it does not have a `Disabled`
    /// component, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and the [`MouseWheel`]. This method can be called repeatedly to register many such
    /// handlers.
    fn on_scroll_with_system_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, MouseWheel)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(ScrollEnabled)
                .observe(|event: On<Add, Disabled>, mut commands: Commands| {
                    if let Ok(mut entity) = commands.get_entity(event.event().entity) {
                        entity.remove::<ScrollEnabled>();
                    }
                })
                .observe(move |event: On<Remove, Disabled>, mut commands: Commands| {
                    if let Ok(mut entity) = commands.get_entity(event.event().entity) {
                        entity.try_insert(ScrollEnabled);
                    }
                })
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |mouse_wheel: On<MouseWheelEvent>, mut commands: Commands| {
                        let MouseWheelEvent { entity, mouse_wheel } = *mouse_wheel.event();
                        commands.run_system_with(system, (entity, mouse_wheel));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When this element receives a [`MouseWheel`] event, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and the [`MouseWheel`]. This method can
    /// be called repeatedly to register many such handlers.
    fn on_scroll_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, MouseWheel)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_scroll_with_system_disableable::<ScrollDisabled, Marker>(handler)
    }

    /// When this element receives a [`MouseWheel`] event, run a system which takes
    /// [`In`](`System::In`) this element's [`Entity`] and the [`MouseWheel`], reactively
    /// controlling whether the handling is disabled with a [`Signal`]. This method can be
    /// called repeatedly to register many such handlers.
    fn on_scroll_with_system_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, MouseWheel)>, (), Marker> + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<ScrollDisabled, _>(blocked.map_true(|_: In<()>| ScrollDisabled::default()))
        })
        .on_scroll_with_system_disableable::<ScrollDisabled, _>(handler)
    }

    /// When this element receives a [`MouseWheel`] event, if it does not have a `Disabled`
    /// component, run a function with the [`MouseWheel`]. This method can be called repeatedly to
    /// register many such handlers.
    fn on_scroll_disableable<Disabled: Component>(
        self,
        mut handler: impl FnMut(MouseWheel) + Send + Sync + 'static,
    ) -> Self {
        self.on_scroll_with_system_disableable::<Disabled, _>(move |In((_, mouse_wheel))| handler(mouse_wheel))
    }

    /// When this element receives a [`MouseWheel`] event, run a function with the [`MouseWheel`].
    /// This method can be called repeatedly to register many such handlers.
    fn on_scroll(self, handler: impl FnMut(MouseWheel) + Send + Sync + 'static) -> Self {
        self.on_scroll_disableable::<ScrollDisabled>(handler)
    }

    /// When this element receives a [`MouseWheel`] event, run a function with the [`MouseWheel`],
    /// reactively controlling whether the handling is disabled with a [`Signal`]. This method can
    /// be called repeatedly to register many such handlers.
    fn on_scroll_disableable_signal(
        self,
        handler: impl FnMut(MouseWheel) + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<ScrollDisabled, _>(blocked.map_true(|_: In<()>| ScrollDisabled::default()))
        })
        .on_scroll_disableable::<ScrollDisabled>(handler)
    }
}

/// Convenience trait for enabling scrollability when hovering over an element.
pub trait OnHoverMouseWheelScrollable: MouseWheelScrollable + PointerEventAware {
    /// When this element receives a [`MouseWheel`] event while it is hovered, if it does not have a
    /// [`ScrollDisabled`] component, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`] and the [`MouseWheel`]. This method can be called repeatedly to
    /// register many such handlers.
    fn on_scroll_with_system_on_hover<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, MouseWheel)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_hovered_change(|In((entity, data)): In<(Entity, HoverData)>, mut commands: Commands| {
            if let Ok(mut entity) = commands.get_entity(entity) {
                if data.hovered {
                    entity.remove::<ScrollDisabled>();
                } else {
                    entity.try_insert(ScrollDisabled);
                }
            }
        })
        .on_scroll_with_system_disableable::<ScrollDisabled, _>(handler)
        .with_builder(|builder| builder.insert(ScrollDisabled))
    }

    /// When this element receives a [`MouseWheel`] event while it is hovered, run a function with
    /// the [`MouseWheel`]. This method can be called repeatedly to register many such handlers.
    fn on_scroll_on_hover(self, mut handler: impl FnMut(MouseWheel) + Send + Sync + 'static) -> Self {
        self.on_scroll_with_system_on_hover::<_>(move |In((_, mouse_wheel))| handler(mouse_wheel))
    }
}

impl<T: PointerEventAware + MouseWheelScrollable> OnHoverMouseWheelScrollable for T {}

/// Event triggered when a mouse wheel event occurs on a scrollable element.
#[derive(EntityEvent)]
pub struct MouseWheelEvent {
    entity: Entity,
    mouse_wheel: MouseWheel,
}

fn scroll_system(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    scroll_listeners: Query<Entity, With<ScrollEnabled>>,
    mut commands: Commands,
) {
    let listeners = scroll_listeners.iter().collect::<Vec<_>>();
    for &event in mouse_wheel_events.read() {
        for &entity in &listeners {
            commands.trigger(MouseWheelEvent {
                entity,
                mouse_wheel: event,
            });
        }
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Component)]
pub enum ScrollDirection {
    Horizontal,
    Vertical,
    Both,
}

impl Default for ScrollDirection {
    fn default() -> Self {
        DEFAULT_SCROLL_DIRECTION
    }
}

/// Component for storing the scroll magnitude (pixels per scroll event).
#[derive(Component, Clone, Copy, Default)]
pub struct ScrollMagnitude(pub f32);

/// Configuration for basic scroll handling. Use with [`BasicScrollHandler::into_system`] to create
/// a scroll handler, or use the component-based approach by inserting [`ScrollDirection`] and
/// [`ScrollMagnitude`] components directly.
#[derive(Default)]
pub struct BasicScrollHandler {
    direction: ScrollDirection,
    magnitude: f32,
}

const DEFAULT_SCROLL_DIRECTION: ScrollDirection = ScrollDirection::Vertical;
const DEFAULT_SCROLL_MAGNITUDE: f32 = 10.;

/// Normalizes the scroll amount based on the scroll unit and the specified magnitude.
pub fn scroll_normalizer(unit: MouseScrollUnit, scroll: f32, magnitude: f32) -> f32 {
    match unit {
        MouseScrollUnit::Line => scroll * magnitude,
        MouseScrollUnit::Pixel => scroll.abs().min(magnitude) * scroll.signum(),
    }
}

impl BasicScrollHandler {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self {
            direction: DEFAULT_SCROLL_DIRECTION,
            magnitude: DEFAULT_SCROLL_MAGNITUDE,
        }
    }

    /// Set the [`ScrollDirection`] of viewport movement in response to mouse wheel events.
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the magnitude (in pixels) of viewport movement in response to mouse wheel events.
    pub fn pixels(mut self, pixels: f32) -> Self {
        self.magnitude = pixels;
        self
    }

    // TODO: is there a better return type for this ?
    /// Convert this [`BasicScrollHandler`] into a function that can be passed as a handler to
    /// `on_scroll_...` methods.
    #[allow(clippy::type_complexity)]
    pub fn into_system(
        self,
    ) -> Box<
        dyn FnMut(In<(Entity, MouseWheel)>, Res<ButtonInput<KeyCode>>, Query<&mut ScrollPosition>)
            + Send
            + Sync
            + 'static,
    > {
        let BasicScrollHandler { direction, magnitude } = self;
        let f = move |In((entity, mouse_wheel)): In<(Entity, MouseWheel)>,
                      keys: Res<ButtonInput<KeyCode>>,
                      mut scroll_positions: Query<&mut ScrollPosition>| {
            let dy = scroll_normalizer(mouse_wheel.unit, mouse_wheel.y, magnitude);
            if let Ok(mut scroll_position) = scroll_positions.get_mut(entity) {
                if matches!(direction, ScrollDirection::Vertical)
                    || matches!(direction, ScrollDirection::Both)
                        && !(keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
                {
                    scroll_position.y -= dy;
                } else if matches!(direction, ScrollDirection::Horizontal)
                    || matches!(direction, ScrollDirection::Both)
                        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
                {
                    scroll_position.x -= dy;
                }
            }
        };
        Box::new(f)
    }

    /// Convert this [`BasicScrollHandler`] into a system that reads scroll settings from
    /// [`ScrollDirection`] and [`ScrollMagnitude`] components, allowing reactive updates.
    /// If the components are not present, default values are used.
    #[allow(clippy::type_complexity)]
    pub fn component_based_system() -> impl FnMut(
        In<(Entity, MouseWheel)>,
        Res<ButtonInput<KeyCode>>,
        Query<&mut ScrollPosition>,
        Query<&ScrollDirection>,
        Query<&ScrollMagnitude>,
    ) + Send
    + Sync
    + 'static {
        move |In((entity, mouse_wheel)): In<(Entity, MouseWheel)>,
              keys: Res<ButtonInput<KeyCode>>,
              mut scroll_positions: Query<&mut ScrollPosition>,
              directions: Query<&ScrollDirection>,
              magnitudes: Query<&ScrollMagnitude>| {
            let direction = directions.get(entity).copied().unwrap_or(DEFAULT_SCROLL_DIRECTION);
            let magnitude = magnitudes.get(entity).map(|m| m.0).unwrap_or(DEFAULT_SCROLL_MAGNITUDE);
            let dy = scroll_normalizer(mouse_wheel.unit, mouse_wheel.y, magnitude);
            if let Ok(mut scroll_position) = scroll_positions.get_mut(entity) {
                if matches!(direction, ScrollDirection::Vertical)
                    || matches!(direction, ScrollDirection::Both)
                        && !(keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
                {
                    scroll_position.y -= dy;
                } else if matches!(direction, ScrollDirection::Horizontal)
                    || matches!(direction, ScrollDirection::Both)
                        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
                {
                    scroll_position.x -= dy;
                }
            }
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, scroll_system.run_if(any_with_component::<ScrollEnabled>));
}
