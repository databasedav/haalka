//! Semantics for managing how an [`Element`](super::element::Element) reacts to pointer events like
//! hover, click, and press, see [`PointerEventAware`].

use std::{
    ops::Not,
    sync::{Arc, OnceLock},
    time::Duration,
};

use apply::Apply;
use bevy_app::prelude::*;
use bevy_derive::*;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_picking::{
    backend::prelude::*,
    hover::{HoverMap, PickingInteraction},
    pointer::PointerMap,
    prelude::*,
};
use bevy_reflect::prelude::*;
use bevy_time::{Time, Timer, TimerMode};
use bevy_window::*;
use jonmo::signal::{Signal, SignalExt};

use super::{
    element::UiRoot,
    global_event_aware::GlobalEventAware,
    utils::{clone, observe, register_system, remove_system_holder_on_remove},
};

/// Helper to create a signal that always outputs a constant value.
fn constant_signal<T: Clone + Send + Sync + 'static>(value: T) -> impl Signal<Item = T> + Send + Sync + 'static {
    SignalBuilder::from_system(move |_: In<()>| Some(value.clone()))
}

use jonmo::signal::SignalBuilder;

/// Enables reacting to pointer events like hover, click, and press. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`PointerEventAware`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/ability/pointer_event_aware.rs).
pub trait PointerEventAware: GlobalEventAware {
    /// When this element's hovered state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and its current hovered state. This method
    /// can be called repeatedly to register many such handlers.
    fn on_hovered_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .insert(Hovered(false))
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |mut enter: On<Pointer<Enter>>, mut commands: Commands| {
                        enter.propagate(false);
                        commands.run_system_with(system, (enter.entity, true));
                    });
                    observe(world, entity, move |mut leave: On<Pointer<Leave>>, mut commands: Commands| {
                        leave.propagate(false);
                        commands.run_system_with(system, (leave.entity, false));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When this element's hover state changes, run a function with its current hovered state. This
    /// method can be called repeatedly to register many such handlers.
    fn on_hovered_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_hovered_change_with_system(move |In((_, is_hovered))| handler(is_hovered))
    }

    /// Run a [`System`] when this element is clicked.
    fn on_click_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |click: On<Pointer<Click>>, mut commands: Commands| {
                        commands.run_system_with(system, (click.entity, (*click).clone()));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// Run a function when this element is left clicked.
    fn on_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_with_system(move |In((_, click)): In<(_, Pointer<Click>)>| {
            if matches!(click.button, PointerButton::Primary) {
                handler()
            }
        })
    }

    /// Run a function when this element is left clicked, reactively controlling whether the click
    /// bubbles up the hierarchy with a [`Signal`].
    fn on_click_propagation_stoppable(
        self,
        mut handler: impl FnMut() + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder
                .insert(Pickable::default())
                .component_signal::<ClickPropagationStopped, _>(
                    propagation_stopped.map_true(|_: In<()>| ClickPropagationStopped),
                )
                .observe(
                    move |mut click: On<Pointer<Click>>, propagation_stopped: Query<&ClickPropagationStopped>| {
                        if propagation_stopped.contains(click.entity) {
                            click.propagate(false);
                        }
                        if matches!(click.button, PointerButton::Primary) {
                            handler()
                        }
                    },
                )
        })
    }

    /// Run a function when this element is left clicked, stopping the click from bubbling up the
    /// hierarchy.
    fn on_click_stop_propagation(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.with_builder(|builder| {
            builder.insert((Pickable::default(), ClickPropagationStopped)).observe(
                move |mut click: On<Pointer<Click>>| {
                    click.propagate(false);
                },
            )
        })
        .on_click(handler)
    }

    /// Run a function when this element is right clicked.
    fn on_right_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_with_system(move |In((_, event)): In<(_, Pointer<Click>)>| {
            if matches!(event.button, PointerButton::Secondary) {
                handler()
            }
        })
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a [`System`] that takes [`In`](`System::In`) this element's
    /// [`Entity`] and the [`Pointer<Click>`]. Will not function unless this element is a descendant
    /// of a [`UiRoot`]. This method can be called repeatedly to register many such handlers.
    #[allow(clippy::type_complexity)]
    fn on_click_outside_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        })
        .on_global_event_with_system::<Pointer<Click>, _>(
            move |In((entity, click)): In<(Entity, Pointer<Click>)>,
                  children: Query<&Children>,
                  child_ofs: Query<&ChildOf>,
                  ui_roots: Query<&UiRoot>,
                  mut commands: Commands| {
                for ancestor in child_ofs.iter_ancestors(entity) {
                    if ui_roots.contains(ancestor) {
                        if !is_inside_or_removed_from_dom(entity, &click, ancestor, &children) {
                            commands.run_system_with(system_holder.get().copied().unwrap(), (entity, click));
                        }
                        break;
                    }
                }
            },
        )
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a function. Will not function unless this element is a descendant of
    /// a [`UiRoot`]. This method can be called repeatedly to register many such handlers.
    fn on_click_outside(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_outside_with_system(move |In((_, _))| handler())
    }

    /// On frames where this element is pressed or gets unpressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`] and its current pressed state. This method can be called repeatedly to register
    /// many such handlers.
    fn on_pressed_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |press: On<Pressed>, blocked: Query<&Blocked>, mut commands: Commands| {
                        let entity = press.entity;
                        if !blocked.contains(entity) {
                            commands.run_system_with(system, (entity, press.event().pressed));
                        }
                    });
                    observe(world, entity, move |pointer_press: On<Pointer<Press>>, mut commands: Commands| {
                        if matches!(pointer_press.button, PointerButton::Primary) {
                            if let Ok(mut entity) = commands.get_entity(pointer_press.entity) {
                                entity.insert(Pressable);
                            }
                        }
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
        .on_hovered_change_with_system(|In((entity, hovered)): In<(_, bool)>, world: &mut World| {
            if !hovered && let Ok(ref mut entity) = world.get_entity_mut(entity) {
                EntityWorldMut::remove::<Pressable>(entity);
            }
        })
    }

    /// When this element's pressed state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and its current pressed state. This method
    /// can be called repeatedly to register many such handlers.
    #[allow(clippy::type_complexity)]
    fn on_pressed_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(clone!(
            (system_holder) | builder | {
                builder.on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
            }
        ))
        .with_builder(remove_system_holder_on_remove(system_holder.clone()))
        .on_pressed_with_system_blockable::<_, PressHandlingBlocked>(
            move |In((entity, cur)), mut pressed: Local<bool>, mut commands: Commands| {
                if cur != *pressed {
                    *pressed = cur;
                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, cur));
                }
            },
        )
    }

    /// When this element's pressed state changes, run a function with its current pressed state.
    fn on_pressed_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_pressed_change_with_system(move |In((_, pressed))| handler(pressed))
    }

    /// On frames where this element is being pressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`]. This method can be called repeatedly to register many such handlers.
    fn on_pressing_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<In<Entity>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(clone!(
            (system_holder) | builder | {
                builder
                    .on_spawn(clone!((system_holder) move |world, _| {
                        let _ = system_holder.set(register_system(world, handler));
                    }))
                    .apply(remove_system_holder_on_remove(system_holder.clone()))
            }
        ))
        .on_pressed_with_system_blockable::<_, Blocked>(
            move |In((entity, pressed)), mut commands: Commands| {
                if pressed {
                    commands.run_system_with(system_holder.get().copied().unwrap(), entity);
                }
            },
        )
    }

    /// On frames where this element is being pressed, run a function.
    fn on_pressing_blockable<Blocked: Component>(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_pressing_with_system_blockable::<_, Blocked>(move |_: In<_>| handler())
    }

    /// On frames where this element is being pressed, run a function, reactively controlling
    /// whether the press is blocked with a [`Signal`].
    fn on_pressing_blockable_signal(
        self,
        handler: impl FnMut() + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<PressHandlingBlocked, _>(
                blocked.map_true(|_: In<()>| PressHandlingBlocked::default()),
            )
        })
        .on_pressing_blockable::<PressHandlingBlocked>(handler)
    }

    /// When this element is being pressed, run a function.
    fn on_pressing(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_pressing_blockable::<PressHandlingBlocked>(handler)
    }

    /// When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`], throttled by `duration` before the `handler` can run again.
    fn on_pressing_with_system_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<Entity>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .insert(PressThrottleTimer(Timer::new(duration, TimerMode::Once)))
                .on_spawn(
                    clone!((system_holder) move |world, _| { let _ = system_holder.set(register_system(world, handler)); }),
                )
                .apply(remove_system_holder_on_remove(system_holder.clone()))
            })
        .on_pressed_with_system_blockable::<_, PressHandlingBlocked>(
            move |In((entity, pressed)), mut commands: Commands, time: Res<Time>, mut timers: Query<&mut PressThrottleTimer>| {
                if pressed {
                    if let Ok(mut timer) = timers.get_mut(entity) {
                        timer.0.tick(time.delta());
                        if timer.0.is_finished() {
                            commands.run_system_with(system_holder.get().copied().unwrap(), entity);
                            timer.0.reset();
                        }
                    }
                }
            },
        )
    }

    /// When this element is being pressed, run a function, throttled by `duration` before the
    /// `handler` can run again.
    fn on_pressing_throttled(self, mut handler: impl FnMut() + Send + Sync + 'static, duration: Duration) -> Self {
        self.on_pressing_with_system_throttled(move |_: In<_>| handler(), duration)
    }
}

/// Timer component for throttling press events.
#[derive(Component, Deref, DerefMut)]
struct PressThrottleTimer(Timer);

#[derive(Component, Deref, DerefMut)]
struct Hovered(bool);

#[derive(Component, Default, Clone)]
struct PressHandlingBlocked;

/// Fires when a the pointer crosses into the bounds of the `target` entity, ignoring children.
#[derive(Clone, PartialEq, Debug, Reflect, Event)]
pub struct Enter {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a the pointer crosses out of the bounds of the `target` entity, excluding children.
#[derive(Clone, PartialEq, Debug, Reflect, Event)]
pub struct Leave {
    // /// Information about the latest prior picking intersection.
    // pub hit: HitData,
}

// TODO: integrate with bubbling observers and upstreamed event listener
fn update_hover_states(
    pointer_map: Res<PointerMap>,
    pointers: Query<&PointerLocation>,
    hover_map: Res<HoverMap>,
    mut hovereds: Query<(Entity, &mut Hovered)>,
    child_ofs: Query<&ChildOf>,
    mut commands: Commands,
) {
    let pointer_id = PointerId::Mouse;
    let hover_set = hover_map.get(&pointer_id);
    for (entity, mut hovered) in hovereds.iter_mut() {
        let hit_data_option = match hover_set {
            Some(map) => map
                .iter()
                .find(|(ha, _)| **ha == entity || child_ofs.iter_ancestors(**ha).any(|e| e == entity))
                .map(|(_, hit_data)| hit_data),
            None => None,
        };
        let is_hovered = hit_data_option.is_some();
        if **hovered != is_hovered {
            **hovered = is_hovered;
            let Some(location) = pointer_map
                .get_entity(pointer_id)
                .and_then(|entity| pointers.get(entity).ok())
                .and_then(|pointer| pointer.location.clone())
            else {
                debug!(
                    "Unable to get location for pointer {:?} during pointer {}",
                    pointer_id,
                    if is_hovered { "enter" } else { "leave" }
                );
                continue;
            };
            if let Some(hit) = hit_data_option.cloned() {
                commands.trigger(Pointer::new(pointer_id, location, Enter { hit }, entity));
            } else {
                // TODO: children `Leave`s don't trigger with this condition, e.g. in an aalo inspector row
                // if let Some(hit) = previous_hover_map
                // .get(&pointer_id)
                // .and_then(|map| map.get(&entity).cloned())
                // {
                // commands.trigger_targets(Pointer::new(pointer_id, location, entity, Leave { hit }), entity);
                commands.trigger(Pointer::new(pointer_id, location, Leave {}, entity));
                // }
            }
        }
    }
}

#[derive(Component, Clone)]
struct ClickPropagationStopped;

#[derive(Component)]
struct OutPropagationStopped;

#[derive(Component)]
struct Pressable;

// TODO: migrate to bevy's Pressed
#[derive(EntityEvent)]
struct Pressed {
    entity: Entity,
    pressed: bool,
}

#[allow(clippy::type_complexity)]
fn pressable_system(
    mut interaction_query: Query<(Entity, &PickingInteraction), (With<Pressable>, Changed<PickingInteraction>)>,
    mut commands: Commands,
) {
    for (entity, interaction) in &mut interaction_query {
        commands.trigger(Pressed {
            entity,
            pressed: matches!(interaction, PickingInteraction::Pressed),
        });
    }
}

fn contains(left: Entity, right: Entity, children_query: &Query<&Children>) -> bool {
    left == right || children_query.iter_descendants(left).any(|e| e == right)
}

// TODO: add support for some sort of exclusion
// ported from moonzoon https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/ability/mouse_event_aware.rs#L158
fn is_inside_or_removed_from_dom(
    element: Entity,
    event: &Pointer<Click>,
    ui_root: Entity,
    children_query: &Query<&Children>,
) -> bool {
    if contains(element, event.entity, children_query) {
        return true;
    }
    if !contains(ui_root, event.entity, children_query) {
        return true;
    }
    false
}

#[derive(Component)]
struct CursorOver;

#[derive(Component, Default, Clone)]
struct CursorDisabled;

/// When this [`Resource`] exists in the [`World`], [`CursorOnHoverable`]
/// [`Element`]s will not trigger updates to the window's cursor when they
/// receive a [`Pointer<Over>`] event. When this [`Resource`] is removed, the last
/// [`Option<CursorIcon>`] queued by a [`CursorOnHoverable`] [`Element`] will be set as the window's
/// cursor. Adding this [`Resource`] to the [`World`] will *not* unset any [`Option<CursorIcon>`]s
/// previously set by a [`CursorOnHoverable`] [`Element`].
///
/// [`Element`]: super::element::Element
#[derive(Resource)]
pub struct CursorOnHoverDisabled;

/// A [`Component`] which stores the [`Option<CursorIcon>`] to set the window's cursor to when an
/// [`Element`](super::element::Element) receives a [`Pointer<Over>`] event; when [`None`], the
/// cursor will be hidden.
#[derive(Component, Clone)]
pub struct CursorOnHover(Option<CursorIcon>);

/// Enables managing the window's [`CursorIcon`] when an [`Element`](super::element::Element)
/// receives an [`Pointer<Over>`] event.
pub trait CursorOnHoverable: PointerEventAware {
    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] in the [`CursorOnHover`] [`Component`] or hide it if [`None`].
    /// If the [`Pointer`] is [`Over`] this element when it is disabled with a `Disabled`
    /// [`Component`], another [`Pointer<Over>`] event will be sent up the hierarchy to trigger
    /// any handlers whose propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_disableable<Disabled: Component>(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        let cursor_option = cursor_option.into();
        self.with_builder(|builder| {
            builder
                .insert((Pickable::default(), CursorOverPropagationStopped))
                .observe(
                    |event: On<Insert, CursorOver>,
                     cursor_on_hovers: Query<&CursorOnHover>,
                     disabled: Query<&Disabled>,
                     cursor_over_disabled_option: Option<Res<CursorOnHoverDisabled>>,
                     mut commands: Commands| {
                        let entity = event.entity;
                        if let Ok(CursorOnHover(cursor_option)) = cursor_on_hovers.get(entity).cloned() {
                            if cursor_over_disabled_option.is_none() {
                                if disabled.contains(entity).not() {
                                    commands.trigger(SetCursor(cursor_option));
                                }
                            } else {
                                commands.insert_resource(QueuedCursor(cursor_option));
                            }
                        }
                    },
                )
                .observe(
                    |event: On<Insert, CursorOnHover>, cursor_overs: Query<&CursorOver>, mut commands: Commands| {
                        let entity = event.entity;
                        if cursor_overs.contains(entity)
                            && let Ok(mut entity) = commands.get_entity(entity)
                        {
                            entity.try_insert(CursorOver);
                        }
                    },
                )
                .insert(CursorOnHover(cursor_option))
                .observe(
                    move |event: On<Add, Disabled>,
                          cursor_over: Query<&CursorOver>,
                          pointer_map: Res<PointerMap>,
                          pointers: Query<&PointerLocation>,
                          hover_map: Res<HoverMap>,
                          mut pointer_over: MessageWriter<Pointer<Over>>,
                          child_ofs: Query<&ChildOf>,
                          mut commands: Commands| {
                        let entity = event.event().entity;
                        if let Ok(mut entity) = commands.get_entity(entity) {
                            entity.remove::<CursorOverPropagationStopped>();
                        }
                        if cursor_over.get(entity).is_ok()
                            && let Some(((hover_map, location), &ChildOf(parent))) = hover_map
                                .get(&PointerId::Mouse)
                                .zip(
                                    pointer_map
                                        .get_entity(PointerId::Mouse)
                                        .and_then(|entity| pointers.get(entity).ok())
                                        .and_then(|pointer| pointer.location.clone()),
                                )
                                .zip(child_ofs.get(entity).ok())
                            && let Some(hit) = hover_map.get(&entity).cloned()
                        {
                            pointer_over.write(Pointer::new(PointerId::Mouse, location, Over { hit }, parent));
                        }
                    },
                )
                .observe(
                    move |event: On<Remove, Disabled>, cursor_over: Query<&CursorOver>, mut commands: Commands| {
                        let entity = event.event().entity;
                        if let Ok(mut entity_commands) = commands.get_entity(entity) {
                            entity_commands.try_insert(CursorOverPropagationStopped);
                            if cursor_over.get(entity).is_ok() {
                                entity_commands.try_insert(CursorOver);
                            }
                        }
                    },
                )
                .observe(
                    |mut over: On<Pointer<Over>>,
                     propagation_stopped: Query<&CursorOverPropagationStopped>,
                     mut commands: Commands| {
                        let entity = over.entity;
                        if propagation_stopped.contains(entity) {
                            over.propagate(false);
                        }
                        if let Ok(mut entity) = commands.get_entity(entity) {
                            entity.try_insert(CursorOver);
                        }
                    },
                )
                .insert(OutPropagationStopped)
                .observe(|mut out: On<Pointer<Out>>, mut commands: Commands| {
                    out.propagate(false);
                    if let Ok(mut entity) = commands.get_entity(out.entity) {
                        entity.remove::<CursorOver>();
                    }
                })
        })
    }

    /// When this [`Element`](super::element::Element) receives a [`Pointer<Over>`] event, set the
    /// window's cursor to [`Some`] [`CursorIcon`] in the [`CursorOnHover`] [`Component`] or
    /// hide it if [`None`].
    fn cursor(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        self.cursor_disableable::<CursorDisabled>(cursor_option)
    }

    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] output by the [`Signal`] or hide it if [`None`]. If the
    /// [`Pointer`] is [`Over`] this element when it is disabled with a `Disabled`
    /// [`Component`], another [`Pointer<Over>`] event will be sent up the hierarchy to trigger
    /// any handlers whose propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_signal_disableable<Disabled: Component>(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<CursorOnHover, _>(cursor_option_signal.map_in(|x| Some(CursorOnHover(x.into()))))
        })
        .cursor_disableable::<Disabled>(None)
    }

    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] output by the [`Signal`] or hide it if [`None`]. If the
    /// [`Pointer`] is [`Over`] this element when it is disabled with the `disabled` [`Signal`]
    /// outputting `true`, another [`Pointer<Over>`] event will be sent up the hierarchy to
    /// trigger any handlers whose propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_signal_disableable_signal(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<CursorDisabled, _>(disabled.map_true(|_: In<()>| CursorDisabled::default()))
        })
        .cursor_signal_disableable::<CursorDisabled>(cursor_option_signal)
    }

    /// When this [`Element`](super::element::Element) receives a [`Pointer<Over>`] event, set the
    /// window's cursor to [`Some`] [`CursorIcon`] output by the [`Signal`] or hide it if
    /// [`None`].
    fn cursor_signal<S: Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static>(
        mut self,
        cursor_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cursor_option_signal) = cursor_option_signal_option.into() {
            self = self.cursor_signal_disableable::<CursorDisabled>(cursor_option_signal);
        }
        self
    }

    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] or hide it if [`None`]. If the [`Pointer`] is [`Over`] this
    /// element when it is disabled with the `disabled` [`Signal`] outputting `true`, another
    /// [`Pointer<Over>`] event will be sent up the hierarchy to trigger any handlers whose
    /// propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_disableable_signal(
        self,
        cursor_option: impl Into<Option<CursorIcon>>,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.cursor_signal_disableable_signal(constant_signal(cursor_option.into()), disabled)
    }
}

/// [`Event`] consumed by a global [`Observer`] to set the window's [`CursorIcon`]; the cursor will
/// be hidden if [`None`].
#[derive(Event)]
pub struct SetCursor(pub Option<CursorIcon>);

#[derive(Component)]
struct CursorOverPropagationStopped;

#[derive(Resource)]
struct QueuedCursor(Option<CursorIcon>);

fn consume_queued_cursor(queued_cursor: Option<Res<QueuedCursor>>, mut commands: Commands) {
    if let Some(cursor) = queued_cursor {
        commands.trigger(SetCursor(cursor.0.clone()));
        commands.remove_resource::<QueuedCursor>();
    }
}

// TODO: add support for multiple windows
fn on_set_cursor(
    event: On<SetCursor>,
    mut cursor_options: Query<(Entity, &mut CursorOptions), With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if let Ok((entity, mut cursor_options)) = cursor_options.single_mut() {
        let SetCursor(icon_option) = event.event();
        if let Some(icon) = icon_option.clone() {
            if let Ok(mut window) = commands.get_entity(entity) {
                window.try_insert(icon);
            }
            cursor_options.visible = true;
        } else {
            cursor_options.visible = false;
        }
    }
}

/// When this [`Resource`] exists in the [`World`], [`Enter`] and [`Leave`] events will not be
/// fired.
#[derive(Resource)]
pub struct UpdateHoverStatesDisabled;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(on_set_cursor).add_systems(
        Update,
        (
            pressable_system.run_if(any_with_component::<Pressable>),
            update_hover_states.run_if(
                any_with_component::<Hovered>
                    // TODO: apparently this updates every frame no matter what, if so, remove this condition
                    // TODO: remove when native `Enter` and `Leave` available
                    .and(resource_exists_and_changed::<HoverMap>)
                    .and(not(resource_exists::<UpdateHoverStatesDisabled>)),
            ),
            consume_queued_cursor.run_if(resource_removed::<CursorOnHoverDisabled>),
        ),
    );
}
