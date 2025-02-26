//! Semantics for managing elements whose contents can be partially visible, see
//! [`ViewportMutable`].

use super::{
    raw::{
        observe, register_system, utils::remove_system_holder_on_remove, RawElWrapper
    },
    utils::clone,
};
use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_hierarchy::prelude::*;
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

/// [`MutableViewport`]s with this [`Component`] receive [`ViewportLocationChange`] events.
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
                .with_component::<Node>(move |mut node| node.overflow = match axis {
                    Axis::Horizontal => Overflow::scroll_x(),
                    Axis::Vertical => Overflow::scroll_y(),
                    Axis::Both => Overflow::scroll(),
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
            let system_holder = Mutable::new(None);
            raw_el
            .insert(OnViewportLocationChange)
            .on_spawn(clone!((system_holder) move |world, entity| {
                let system = register_system(world, handler);
                system_holder.set(Some(system));
                observe(world, entity, move |viewport_location_change: Trigger<ViewportLocationChange>, mut commands: Commands| {
                    let &ViewportLocationChange { scene, viewport } = viewport_location_change.event();
                    commands.run_system_with_input(system, (entity, (scene, viewport)));
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
                .on_signal_with_component::<_, ScrollPosition>(x_signal, |mut scroll_position, x| {
                    scroll_position.offset_x = x;
                })
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
                .on_signal_with_component::<_, ScrollPosition>(y_signal, |mut scroll_position, y| {
                    scroll_position.offset_y = y;
                })
            });
        }
        self
    }
}

// TODO: should not fire when scrolling doesn't actually change the viewport
/// [`Trigger`]ed when the [`Viewport`] or [`Scene`] of a [`MutableViewport`] changes; only entities
/// with the [`OnViewportLocationChange`] component receive this event.
#[derive(Event)]
pub struct ViewportLocationChange {
    #[allow(missing_docs)]
    pub scene: Scene,
    #[allow(missing_docs)]
    pub viewport: Viewport,
}

// #[allow(clippy::type_complexity)]
// fn scene_change_dispatcher(
//     mut data: Query<
//         (Entity, &ComputedNode, &Node, &mut MutableViewport),
//         (With<OnViewportLocationChange>, Or<(Changed<Node>, Changed<Transform>)>),
//     >,
//     mut commands: Commands,
// ) {
//     for (entity, computed_node, node, mut mutable_viewport) in data.iter_mut() {
//         let Vec2 { x, y } = computed_node.size();
//         mutable_viewport.scene.width = x;
//         mutable_viewport.scene.height = y;
//         if let Val::Px(x) = node.left {
//             mutable_viewport.viewport.x = -x;
//         }
//         if let Val::Px(y) = node.top {
//             mutable_viewport.viewport.y = -y;
//         }
//         let MutableViewport { scene, viewport, .. } = *mutable_viewport;
//         commands.trigger_targets(ViewportLocationChange { scene, viewport }, entity);
//     }
// }

// #[allow(clippy::type_complexity)]
// fn viewport_change_dispatcher(
//     data: Query<(Entity, &ComputedNode), (With<ViewportMarker>, Changed<ComputedNode>)>,
//     children: Query<&Children>,
//     mut mutable_viewports: Query<&mut MutableViewport, With<OnViewportLocationChange>>,
//     mut commands: Commands,
// ) {
//     for (entity, computed_node) in data.iter() {
//         let Vec2 { x, y } = computed_node.size();
//         // [`Scene`] is the [`Viewport`]'s only child
//         if let Some(&child) = firstborn(entity, &children) {
//             if let Ok(mut mutable_viewport) = mutable_viewports.get_mut(child) {
//                 mutable_viewport.viewport.width = x;
//                 mutable_viewport.viewport.height = y;
//                 let MutableViewport { scene, viewport, .. } = *mutable_viewport;
//                 commands.trigger_targets(ViewportLocationChange { scene, viewport }, child);
//             }
//         }
//     }
// }

/// Use to fetch the logical pixel coordinates of the UI node, based on its [`GlobalTransform`].
#[derive(SystemParam)]
pub struct LogicalRect<'w, 's> {
    data: Query<'w, 's, (&'static ComputedNode, &'static GlobalTransform)>,
}

impl<'w, 's> LogicalRect<'w, 's> {
    fn get(&self, entity: Entity) -> Option<Rect> {
        if let Ok((computed_node, global_transform)) = self.data.get(entity) {
            return Rect::from_center_size(
                global_transform.translation().xy(),
                computed_node.size(),
            )
            .apply(Some);
        }
        None
    }
}

fn viewport_location_change_dispatcher(
    data: Query<(Entity, &ScrollPosition), (Or<(Changed<ComputedNode>, Changed<ScrollPosition>)>, With<OnViewportLocationChange>)>,
    children: Query<&Children>,
    logical_rect: LogicalRect,
    mut commands: Commands,
) {
    for (entity, scroll_position) in data.iter() {
        if let Some(Vec2 { x: viewport_width, y: viewport_height }) = logical_rect.get(entity).as_ref().map(Rect::size) {
            let ScrollPosition { offset_x, offset_y } = *scroll_position;
            let mut min = Vec2::MAX;
            let mut max = Vec2::MIN;
            for child in children.get(entity).ok().into_iter().flat_map(|children| children.iter()) {
                if let Some(child_rect) = logical_rect.get(*child) {
                    min = min.min(child_rect.min);
                    max = max.max(child_rect.max);
                }
            }
            commands.trigger_targets(
                ViewportLocationChange {
                    scene: Scene { width: max.x - min.x, height: max.y - min.y },
                    viewport: Viewport {
                        offset_x,
                        offset_y,
                        width: viewport_width,
                        height: viewport_height,
                    },
                },
                entity,
            );
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, viewport_location_change_dispatcher);
    // app.add_systems(
    //     Update,
    //     (scene_change_dispatcher, viewport_change_dispatcher)
    //         .run_if(any_with_component::<MutableViewport>.and(any_with_component::<OnViewportLocationChange>)),
    // );
}

pub(crate) fn firstborn<'a>(entity: Entity, children: &'a Query<&Children>) -> Option<&'a Entity> {
    children.get(entity).ok().and_then(|children| children.first())
}
