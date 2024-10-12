use super::raw::{DeferredUpdaterAppendDirection, RawElWrapper, RawHaalkaEl};
use bevy::prelude::*;
use futures_signals::signal::Signal;

pub enum LimitToBody {
    Horizontal,
    Vertical,
    Both,
}

// can also be used to query for mutable viewports
#[derive(Component)]
pub struct MutableViewportSettings {
    limit_to_body: Option<LimitToBody>,
}

// pub struct Viewport {
//     pub x: f32,
//     pub y: f32,
//     width: f32,
//     height: f32,
// }

// pub struct Scene {
//     pub width: f32,
//     pub height: f32,
// }

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
                .insert(MutableViewportSettings { limit_to_body })
                .observe(
                    move |mutation: Trigger<ViewportMutation>,
                          mut styles: Query<&mut Style>,
                          parents: Query<&Parent>,
                          nodes: Query<&Node>,
                          settings: Query<&MutableViewportSettings>| {
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
                    // this wrapper element is the [`Viewport`]
                    RawHaalkaEl::from(NodeBundle::default())
                        .with_component::<Style>(move |mut style| {
                            style.display = Display::Flex;
                            style.overflow = overflow;
                        })
                        .child(raw_el) // the `raw_el` here is the [`Scene`]
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

    // TODO
    // fn on_viewport_location_change(self, mut handler: impl FnMut(Scene, Viewport) + 'static) -> Self

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
