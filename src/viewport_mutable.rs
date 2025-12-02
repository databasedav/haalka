//! Semantics for managing elements whose contents can be partially visible, see
//! [`ViewportMutable`].

use std::{
    collections::HashSet,
    sync::{Arc, OnceLock},
};

use super::{
    raw::{RawElWrapper, observe, register_system, utils::remove_system_holder_on_remove},
    utils::clone,
};
use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::prelude::*;
use bevy_transform::prelude::*;
use bevy_ui::prelude::*;
use futures_signals::signal::{Mutable, Signal};

/// Dimensions of an element's "scene", which contains both its visible (via its [`Viewport`]) and
/// hidden parts.
#[derive(Clone, Copy, Default, Debug)]
pub struct Scene {
    #[allow(missing_docs)]
    pub width: f32,
    #[allow(missing_docs)]
    pub height: f32,
}

/// Data specifying the visible portion of an element's [`Scene`].
#[derive(Clone, Copy, Default, Debug)]
pub struct Viewport {
    /// Horizontal offset.
    pub offset_x: f32,
    /// Vertical offset.
    pub offset_y: f32,
    #[allow(missing_docs)]
    pub width: f32,
    #[allow(missing_docs)]
    pub height: f32,
}

// TODO: should not fire when scrolling doesn't actually change the viewport
/// [`Component`] for holding the [`Scene`] and [`Viewport`]. Also an [`Event`] which is
/// [`Trigger`]ed when the [`Viewport`] or [`Scene`] of a [`MutableViewport`] changes; only entities
/// with the [`OnViewportLocationChange`] component receive this event.
#[derive(Component, Event, Default)]
pub struct MutableViewport {
    #[allow(missing_docs)]
    pub scene: Scene,
    #[allow(missing_docs)]
    pub viewport: Viewport,
}

#[derive(EntityEvent)]
struct MutableViewportEvent {
    entity: Entity,
    mutable_viewport: MutableViewport,
}

/// [`MutableViewport`]s with this [`Component`] receive [`MutableViewport`] events.
#[derive(Component)]
pub struct OnViewportLocationChange;

/// Along which axes the [`Viewport`] can be mutated.
pub enum Axis {
    #[allow(missing_docs)]
    Horizontal,
    #[allow(missing_docs)]
    Vertical,
    #[allow(missing_docs)]
    Both,
}

/// Sentinel component to store the last scroll position set by a signal.
/// This is used to break feedback loops in two-way bindings.
#[derive(Component, Default, Debug)]
struct LastSignalScrollPosition {
    x: f32,
    y: f32,
}

/// Enables the management of a limited visible window (viewport) onto the body of an element.
/// CRITICALLY NOTE that methods expecting viewport mutability will not function without calling
/// [`.mutable_viewport(...)`](ViewportMutable::mutable_viewport).
pub trait ViewportMutable: RawElWrapper {
    /// CRITICALLY NOTE, methods expecting viewport mutability will not function without calling
    /// this method. I could not find a way to enforce this at compile time; please let me know if
    /// you can.
    fn mutable_viewport(self, axis: Axis) -> Self {
        self.update_raw_el(move |raw_el| {
            raw_el
                .insert(MutableViewport::default())
                .with_component::<Node>(move |mut node| {
                    node.overflow = match axis {
                        Axis::Horizontal => Overflow::scroll_x(),
                        Axis::Vertical => Overflow::scroll_y(),
                        Axis::Both => Overflow::scroll(),
                    }
                })
        })
    }

    /// When this element's [`Scene`] or [`Viewport`] changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`], [`Scene`], and [`Viewport`]. This method
    /// can be called repeatedly to register many such handlers.
    fn on_viewport_location_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, (Scene, Viewport))>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Arc::new(OnceLock::new());
            raw_el
            .insert(OnViewportLocationChange)
            .on_spawn(clone!((system_holder) move |world, entity| {
                let system = register_system(world, handler);
                let _ = system_holder.set(system);
                observe(world, entity, move |viewport_location_change: On<MutableViewportEvent>, mut commands: Commands| {
                    let &MutableViewportEvent { mutable_viewport: MutableViewport { scene, viewport }, .. } = viewport_location_change.event();
                    commands.run_system_with(system, (entity, (scene, viewport)));
                });
            }))
            .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When this element's [`Scene`] or [`Viewport`] changes, run a function with its [`Scene`] and
    /// [`Viewport`]. This method can be called repeatedly to register many such handlers.
    fn on_viewport_location_change(self, mut handler: impl FnMut(Scene, Viewport) + Send + Sync + 'static) -> Self {
        self.on_viewport_location_change_with_system(move |In((_, (scene, viewport)))| handler(scene, viewport))
    }

    /// Reactively set the horizontal position of the viewport.
    fn viewport_x_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        x_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(x_signal) = x_signal_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el
                    .insert(LastSignalScrollPosition::default())
                    .on_signal_with_system(
                    x_signal,
                    |In((entity, x)): In<(Entity, f32)>,
                     mut query: Query<(&mut ScrollPosition, &mut LastSignalScrollPosition)>| {
                        if let Ok((mut scroll_pos, mut last_signal_pos)) = query.get_mut(entity)
                            && last_signal_pos.x.to_bits() != x.to_bits()
                        {
                            last_signal_pos.x = x;
                            scroll_pos.x = x;
                        }
                    },
                )
            });
        }
        self
    }

    /// Reactively set the vertical position of the viewport.
    fn viewport_y_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        y_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(y_signal) = y_signal_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el
                    .insert(LastSignalScrollPosition::default())
                    .on_signal_with_system(
                    y_signal,
                    |In((entity, y)): In<(Entity, f32)>,
                     mut query: Query<(&mut ScrollPosition, &mut LastSignalScrollPosition)>| {
                        if let Ok((mut scroll_pos, mut last_signal_pos)) = query.get_mut(entity)
                            && last_signal_pos.y.to_bits() != y.to_bits()
                        {
                            last_signal_pos.y = y;
                            scroll_pos.y = y;
                        }
                    },
                )
            });
        }
        self
    }

    /// Sync a [`Mutable<f32>`] with this element's viewport's x offset.
    fn viewport_x_sync(self, viewport_x: Mutable<f32>) -> Self {
        self.on_viewport_location_change_with_system(
            move |In((entity, (_, viewport))): In<(Entity, (Scene, Viewport))>,
                  last_signal_positions: Query<&LastSignalScrollPosition>| {
                if let Ok(last_signal_pos) = last_signal_positions.get(entity)
                    && last_signal_pos.x.to_bits() != viewport.offset_x.to_bits()
                {
                    viewport_x.set_neq(viewport.offset_x);
                }
            },
        )
    }

    /// Sync a [`Mutable<f32>`] with this element's viewport's y offset.
    fn viewport_y_sync(self, viewport_y: Mutable<f32>) -> Self {
        self.on_viewport_location_change_with_system(
            move |In((entity, (_, viewport))): In<(Entity, (Scene, Viewport))>,
                  last_signal_positions: Query<&LastSignalScrollPosition>| {
                if let Ok(last_signal_pos) = last_signal_positions.get(entity)
                    && last_signal_pos.y.to_bits() != viewport.offset_y.to_bits()
                {
                    viewport_y.set_neq(viewport.offset_y);
                }
            },
        )
    }
}

/// Use to fetch the logical pixel coordinates of the UI node, based on its [`GlobalTransform`].
#[derive(SystemParam)]
pub struct LogicalRect<'w, 's> {
    data: Query<'w, 's, (&'static ComputedNode, &'static GlobalTransform)>,
}

impl LogicalRect<'_, '_> {
    /// Get the logical pixel coordinates of the UI node, based on its [`GlobalTransform`].
    pub fn get(&self, entity: Entity) -> Option<Rect> {
        if let Ok((computed_node, global_transform)) = self.data.get(entity) {
            return Rect::from_center_size(global_transform.translation().xy(), computed_node.size()).apply(Some);
        }
        None
    }
}

#[derive(SystemParam)]
struct SceneViewport<'w, 's> {
    childrens: Query<'w, 's, &'static Children>,
    logical_rect: LogicalRect<'w, 's>,
    scroll_positions: Query<'w, 's, &'static ScrollPosition>,
}

impl SceneViewport<'_, '_> {
    fn get(&self, entity: Entity) -> Option<(Scene, Viewport)> {
        if let Some(Vec2 {
            x: viewport_width,
            y: viewport_height,
        }) = self.logical_rect.get(entity).as_ref().map(Rect::size)
            && let Ok(&ScrollPosition(Vec2 { x: offset_x, y: offset_y })) = self.scroll_positions.get(entity)
        {
            let mut min = Vec2::MAX;
            let mut max = Vec2::MIN;
            for child in self
                .childrens
                .get(entity)
                .ok()
                .into_iter()
                .flat_map(|children| children.iter())
            {
                if let Some(child_rect) = self.logical_rect.get(child) {
                    min = min.min(child_rect.min);
                    max = max.max(child_rect.max);
                }
            }
            let scene = Scene {
                width: max.x - min.x,
                height: max.y - min.y,
            };
            let viewport = Viewport {
                offset_x,
                offset_y,
                width: viewport_width,
                height: viewport_height,
            };
            return Some((scene, viewport));
        }
        None
    }
}

fn dispatch_viewport_location_change(
    entity: Entity,
    scene_viewports: &SceneViewport,
    commands: &mut Commands,
    checked_viewport_listeners: &mut HashSet<Entity>,
) {
    if let Some((scene, viewport)) = scene_viewports.get(entity) {
        if let Ok(mut entity) = commands.get_entity(entity) {
            entity.insert(MutableViewport { scene, viewport });
        }
        commands.trigger(MutableViewportEvent { entity, mutable_viewport: MutableViewport { scene, viewport } });
        checked_viewport_listeners.insert(entity);
    }
}

#[allow(clippy::type_complexity)]
fn viewport_location_change_dispatcher(
    viewports: Query<
        Entity,
        (
            Or<(Changed<ComputedNode>, Changed<ScrollPosition>, Changed<Children>)>,
            With<OnViewportLocationChange>,
        ),
    >,
    changed_computed_nodes: Query<Entity, Changed<ComputedNode>>,
    viewport_location_change_listeners: Query<Entity, With<OnViewportLocationChange>>,
    child_ofs: Query<&ChildOf>,
    scene_viewports: SceneViewport,
    mut commands: Commands,
) {
    let mut checked_viewport_listeners = HashSet::new();
    for entity in viewports.iter() {
        dispatch_viewport_location_change(entity, &scene_viewports, &mut commands, &mut checked_viewport_listeners);
    }
    for entity in changed_computed_nodes.iter() {
        if let Ok(&ChildOf(parent)) = child_ofs.get(entity)
            && !checked_viewport_listeners.contains(&parent)
            && viewport_location_change_listeners.contains(parent)
        {
            dispatch_viewport_location_change(parent, &scene_viewports, &mut commands, &mut checked_viewport_listeners);
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        viewport_location_change_dispatcher.run_if(any_with_component::<OnViewportLocationChange>),
    );
}
