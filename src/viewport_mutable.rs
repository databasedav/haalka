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

#[derive(Clone, Copy)]
pub enum LimitToBody {
    Horizontal,
    Vertical,
    Both,
}

// can also be used to query for mutable viewports
#[derive(Component)]
pub struct MutableViewport {
    limit_to_body: Option<LimitToBody>,
    scene: Scene,
    viewport: Viewport,
}

impl MutableViewport {
    pub fn new(limit_to_body: Option<LimitToBody>) -> Self {
        Self {
            limit_to_body,
            scene: default(),
            viewport: default(),
        }
    }

    pub fn scene(&self) -> Scene {
        self.scene
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
}

#[derive(Component)]
pub struct ViewportMarker;

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

#[derive(Clone, Copy, Default, Debug)]
pub struct Scene {
    pub width: f32,
    pub height: f32,
}

#[derive(Default, Event)]
pub struct ViewportMutation {
    x: Option<f32>,
    y: Option<f32>,
}

impl ViewportMutation {
    pub fn x(x: f32) -> Self {
        Self::default().with_x(x)
    }

    pub fn y(y: f32) -> Self {
        Self::default().with_y(y)
    }

    pub fn with_x(mut self, x: f32) -> Self {
        self.x = Some(x);
        self
    }

    pub fn with_y(mut self, y: f32) -> Self {
        self.y = Some(y);
        self
    }
}

#[derive(Component)]
pub struct OnViewportLocationChange;

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
                          mut styles: Query<&mut Style>,
                          parents: Query<&Parent>,
                          nodes: Query<&Node>,
                          settings: Query<&MutableViewport>| {
                        let entity = mutation.entity();
                        if let Some((((node, parent), settings), mut style)) = nodes
                            .get(entity)
                            .ok()
                            .zip(parents.get(entity).ok().and_then(|parent| nodes.get(parent.get()).ok()))
                            .zip(settings.get(entity).ok())
                            .zip(styles.get_mut(entity).ok())
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
                                    x = x.clamp(-(node.size().x - parent.size().x).max(0.), 0.)
                                };
                                style.left = Val::Px(x);
                            }
                            if let Some(mut y) = y_option {
                                if matches!(
                                    settings.limit_to_body,
                                    Some(LimitToBody::Vertical) | Some(LimitToBody::Both)
                                ) {
                                    y = y.clamp(-(node.size().y - parent.size().y).max(0.), 0.);
                                };
                                style.top = Val::Px(y);
                            }
                        }
                    },
                )
                .defer_update(DeferredUpdaterAppendDirection::Front, move |raw_el| {
                    RawHaalkaEl::from(NodeBundle::default())
                        .insert(ViewportMarker)
                        .with_component::<Style>(move |mut style| {
                            style.display = Display::Flex;
                            style.overflow = overflow;
                        })
                        .child(raw_el) // this is the [`Scene`]
                        .on_spawn_with_system(
                            |In(entity), children: Query<&Children>, mut styles: Query<&mut Style>| {
                                // match the flex direction of `raw_el` above
                                if let Ok(children) = children.get(entity) {
                                    if let Some(&child) = children.first() {
                                        if let Some((flex_direction, mut style)) = styles
                                            .get(child)
                                            .map(|style| style.flex_direction)
                                            .ok()
                                            .zip(styles.get_mut(entity).ok())
                                        {
                                            style.flex_direction = flex_direction;
                                        }
                                    }
                                }
                            },
                        )
                })
        })
    }

    fn on_viewport_location_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, (Scene, Viewport)), (), Marker> + Send + 'static,
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
    mut data: Query<(Entity, &Node, &Style, &mut MutableViewport), Or<(Changed<Node>, Changed<Transform>)>>,
    mut commands: Commands,
) {
    for (entity, node, style, mut mutable_viewport) in data.iter_mut() {
        let Vec2 { x, y } = node.size();
        mutable_viewport.scene.width = x;
        mutable_viewport.scene.height = y;
        if let Val::Px(x) = style.left {
            mutable_viewport.viewport.x = -x;
        }
        if let Val::Px(y) = style.top {
            mutable_viewport.viewport.y = -y;
        }
        let MutableViewport { scene, viewport, .. } = *mutable_viewport;
        commands.trigger_targets(ViewportLocationChange { scene, viewport }, entity);
    }
}

#[allow(clippy::type_complexity)]
fn viewport_change_dispatcher(
    data: Query<(Entity, &Node), (With<ViewportMarker>, Changed<Node>)>,
    children: Query<&Children>,
    mut mutable_viewports: Query<&mut MutableViewport>,
    mut commands: Commands,
) {
    for (entity, node) in data.iter() {
        let Vec2 { x, y } = node.size();
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
            .run_if(any_with_component::<MutableViewport>.and_then(any_with_component::<OnViewportLocationChange>)),
    );
}
