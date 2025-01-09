//! Semantics for managing elements whose contents can be partially visible, see
//! [`ViewportMutable`].

use crate::raw::{observe, register_system};

use super::{
    raw::{utils::remove_system_holder_on_remove, DeferredUpdaterAppendDirection, RawElWrapper, RawHaalkaEl},
    utils::clone,
};
use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::prelude::*;
use bevy_ui::prelude::*;
use bevy_utils::prelude::*;
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
    pub x: f32,
    /// Vertical offset.
    pub y: f32,
    #[allow(missing_docs)]
    pub width: f32,
    #[allow(missing_docs)]
    pub height: f32,
}

/// Specifies which axes viewport mutation is limited to the body of the [`Scene`].
///
/// For example, if no [`LimitToBody`] were specified, one could mutate the viewport (e.g. scroll)
/// past the actual content of the element.
#[derive(Clone, Copy)]
pub enum LimitToBody {
    #[allow(missing_docs)]
    Horizontal,
    #[allow(missing_docs)]
    Vertical,
    #[allow(missing_docs)]
    Both,
}

/// [`Component`] for holding the [`Scene`], [`Viewport`], and relavant configuration.
#[derive(Component)]
pub struct MutableViewport {
    scene: Scene,
    viewport: Viewport,
    limit_to_body: Option<LimitToBody>,
}

impl MutableViewport {
    #[allow(missing_docs)]
    pub fn new(limit_to_body: Option<LimitToBody>) -> Self {
        Self {
            limit_to_body,
            scene: default(),
            viewport: default(),
        }
    }

    #[allow(missing_docs)]
    pub fn scene(&self) -> Scene {
        self.scene
    }

    #[allow(missing_docs)]
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
}

/// Marker [`Component`] for identifying the [`Viewport`] [`Entity`] of a [`MutableViewport`].
#[derive(Component)]
pub struct ViewportMarker;

/// Event for modifying the horizontal and vertical offset of the [`Viewport`].
#[derive(Default, Event)]
pub struct ViewportMutation {
    /// Optional horizontal offset mutation.
    x: Option<f32>,
    /// Optional vertical offset mutation.
    y: Option<f32>,
}

impl ViewportMutation {
    #[allow(missing_docs)]
    pub fn x(x: f32) -> Self {
        Self::default().with_x(x)
    }

    #[allow(missing_docs)]
    pub fn y(y: f32) -> Self {
        Self::default().with_y(y)
    }

    #[allow(missing_docs)]
    pub fn with_x(mut self, x: f32) -> Self {
        self.x = Some(x);
        self
    }

    #[allow(missing_docs)]
    pub fn with_y(mut self, y: f32) -> Self {
        self.y = Some(y);
        self
    }
}

#[derive(Component)]
struct OnViewportLocationChange;

/// Enables the management of a limited visible window (viewport) onto the body of an element.
/// CRITICALLY NOTE that methods expecting viewport mutability will not function without calling
/// [`.mutable_viewport(...)`](ViewportMutable::mutable_viewport).
pub trait ViewportMutable: RawElWrapper {
    /// CRITICALLY NOTE, methods expecting viewport mutability will not function without calling
    /// this method. I could not find a way to enforce this at compile time; please let me know if
    /// you can.
    ///
    /// # Arguments
    /// * `overflow` - [`Overflow`] setting for the wrapping container.
    /// * `limit_to_body` - Whether to clamp scrolling to the body of the element on [`Some`] or
    ///   [`None`] of its axes.
    fn mutable_viewport(self, overflow: Overflow, limit_to_body: impl Into<Option<LimitToBody>>) -> Self {
        let limit_to_body = limit_to_body.into();
        self.update_raw_el(move |raw_el| {
            raw_el
                .insert(MutableViewport::new(limit_to_body))
                // .observe(observer)
                .observe(
                    move |mutation: Trigger<ViewportMutation>,
                          mut nodes: Query<&mut Node>,
                          parents: Query<&Parent>,
                          computed_nodes: Query<&ComputedNode>,
                          settings: Query<&MutableViewport>| {
                        let entity = mutation.entity();
                        if let Some((((computed_node, parent), settings), mut node)) = computed_nodes
                            .get(entity)
                            .ok()
                            .zip(
                                parents
                                    .get(entity)
                                    .ok()
                                    .and_then(|parent| computed_nodes.get(parent.get()).ok()),
                            )
                            .zip(settings.get(entity).ok())
                            .zip(nodes.get_mut(entity).ok())
                        {
                            let &ViewportMutation {
                                x: x_option,
                                y: y_option,
                            } = mutation.event();
                            if let Some(mut x) = x_option {
                                if matches!(
                                    settings.limit_to_body,
                                    Some(LimitToBody::Horizontal) | Some(LimitToBody::Both)
                                ) {
                                    x = x.clamp(-(computed_node.size().x - parent.size().x).max(0.), 0.)
                                };
                                node.left = Val::Px(x);
                            }
                            if let Some(mut y) = y_option {
                                if matches!(
                                    settings.limit_to_body,
                                    Some(LimitToBody::Vertical) | Some(LimitToBody::Both)
                                ) {
                                    y = y.clamp(-(computed_node.size().y - parent.size().y).max(0.), 0.);
                                };
                                node.top = Val::Px(y);
                            }
                        }
                    },
                )
                .defer_update(DeferredUpdaterAppendDirection::Front, move |raw_el| {
                    RawHaalkaEl::from(Node {
                        display: Display::Flex,
                        overflow,
                        ..default()
                    })
                    .insert(ViewportMarker)
                    .child(raw_el) // this is the [`Scene`]
                    .on_spawn_with_system(
                        |In(entity), children: Query<&Children>, mut nodes: Query<&mut Node>| {
                            // match the flex direction of `raw_el` above
                            if let Ok(children) = children.get(entity) {
                                if let Some(&child) = children.first() {
                                    if let Some((flex_direction, mut node)) = nodes
                                        .get(child)
                                        .map(|node| node.flex_direction)
                                        .ok()
                                        .zip(nodes.get_mut(entity).ok())
                                    {
                                        node.flex_direction = flex_direction;
                                    }
                                }
                            }
                        },
                    )
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
                raw_el.on_signal_one_shot(
                    x_signal,
                    |In((entity, x)): In<(Entity, f32)>, mut commands: Commands| {
                        commands.trigger_targets(ViewportMutation::x(x), entity);
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
                raw_el.on_signal_one_shot(
                    y_signal,
                    |In((entity, y)): In<(Entity, f32)>, mut commands: Commands| {
                        commands.trigger_targets(ViewportMutation::y(y), entity);
                    },
                )
            });
        }
        self
    }
}

#[derive(Event)]
struct ViewportLocationChange {
    scene: Scene,
    viewport: Viewport,
}

#[allow(clippy::type_complexity)]
fn scene_change_dispatcher(
    mut data: Query<(Entity, &ComputedNode, &Node, &mut MutableViewport), Or<(Changed<Node>, Changed<Transform>)>>,
    mut commands: Commands,
) {
    for (entity, computed_node, node, mut mutable_viewport) in data.iter_mut() {
        let Vec2 { x, y } = computed_node.size();
        mutable_viewport.scene.width = x;
        mutable_viewport.scene.height = y;
        if let Val::Px(x) = node.left {
            mutable_viewport.viewport.x = -x;
        }
        if let Val::Px(y) = node.top {
            mutable_viewport.viewport.y = -y;
        }
        let MutableViewport { scene, viewport, .. } = *mutable_viewport;
        commands.trigger_targets(ViewportLocationChange { scene, viewport }, entity);
    }
}

#[allow(clippy::type_complexity)]
fn viewport_change_dispatcher(
    data: Query<(Entity, &ComputedNode), (With<ViewportMarker>, Changed<ComputedNode>)>,
    children: Query<&Children>,
    mut mutable_viewports: Query<&mut MutableViewport>,
    mut commands: Commands,
) {
    for (entity, computed_node) in data.iter() {
        let Vec2 { x, y } = computed_node.size();
        if let Ok(children) = children.get(entity) {
            // [`Scene`] is the [`Viewport`]'s only child
            if let Some(&child) = children.first() {
                if let Ok(mut mutable_viewport) = mutable_viewports.get_mut(child) {
                    mutable_viewport.viewport.width = x;
                    mutable_viewport.viewport.height = y;
                    let MutableViewport { scene, viewport, .. } = *mutable_viewport;
                    commands.trigger_targets(ViewportLocationChange { scene, viewport }, child);
                }
            }
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (scene_change_dispatcher, viewport_change_dispatcher)
            .run_if(any_with_component::<MutableViewport>.and(any_with_component::<OnViewportLocationChange>)),
    );
}
