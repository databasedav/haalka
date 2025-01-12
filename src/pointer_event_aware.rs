//! Semantics for managing how an [`Element`](super::element::Element) reacts to pointer events like
//! hover, click, and press, see [`PointerEventAware`].

use std::{future::Future, ops::Not, time::Duration};

use apply::Apply;
use bevy_app::prelude::*;
use bevy_derive::*;
use bevy_ecs::{prelude::*, system::*};
use bevy_hierarchy::prelude::*;
use bevy_log::prelude::*;
use bevy_picking::{
    backend::prelude::*,
    focus::{HoverMap, PickingInteraction},
    pointer::PointerMap,
    prelude::*,
};
use bevy_reflect::prelude::*;
use bevy_utils::prelude::*;
use bevy_window::{prelude::*, *};
use bevy_winit::cursor::CursorIcon;
use enclose::enclose as clone;
use futures_signals::signal::{always, channel, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::SignalExtBool;

use super::{
    element::UiRoot,
    raw::{observe, register_system, utils::remove_system_holder_on_remove, RawElWrapper},
    utils::sleep,
};

/// Enables reacting to pointer events like hover, click, and press. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`PointerEventAware`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/ability/pointer_event_aware.rs).
pub trait PointerEventAware: RawElWrapper {
    /// When this element's hovered state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and its current hovered state. This method
    /// can be called repeatedly to register many such handlers.
    fn on_hovered_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.defer_update(super::raw::DeferredUpdaterAppendDirection::Back, |raw_el| {
                let system_holder = Mutable::new(None);
                raw_el
                    .insert(PickingBehavior::default())
                    .insert(Hovered(false))
                    .on_spawn(clone!((system_holder) move |world, entity| {
                        let system = register_system(world, handler);
                        system_holder.set(Some(system));
                        observe(world, entity, move |enter: Trigger<Pointer<Enter>>, mut commands: Commands| commands.run_system_with_input(system, (enter.entity(), true)));
                        observe(world, entity, move |leave: Trigger<Pointer<Leave>>, mut commands: Commands| commands.run_system_with_input(system, (leave.entity(), false)));
                    }))
                    .apply(remove_system_holder_on_remove(system_holder))
            })
        })
    }

    /// When this element's hover state changes, run a function with its current hovered state. This
    /// method can be called repeatedly to register many such handlers.
    fn on_hovered_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_hovered_change_with_system(move |In((_, is_hovered))| handler(is_hovered))
    }

    /// Sync a [`Mutable<bool>`] with this element's hovered state.
    fn hovered_sync(self, hovered: Mutable<bool>) -> Self {
        self.on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    }

    /// Run a [`System`] when this element is clicked.
    fn on_click_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(PickingBehavior::default())
                .on_event_with_system::<Pointer<Click>, _>(handler)
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
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(PickingBehavior::default())
                .on_event_propagation_stoppable_signal::<Pointer<Click>>(
                    move |click| {
                        if matches!(click.button, PointerButton::Primary) {
                            handler()
                        }
                    },
                    propagation_stopped,
                )
        })
    }

    /// Run a function when this element is left clicked, stopping the click from bubbling up the
    /// hierarchy.
    fn on_click_stop_propagation(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_propagation_stoppable(handler, always(true))
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
    /// [`Entity`] and the [`Pointer<Click>`]. Requires the [`UiRoot`] [`Resource`] to exist in the
    /// [`World`]. This method can be called repeatedly to register many such handlers.
    fn on_click_outside_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .insert(OnClickOutside)
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    system_holder.set(Some(system));
                    observe(world, entity, move |click_outside: Trigger<ClickOutside>, mut commands: Commands| {
                        commands.run_system_with_input(system, (entity, click_outside.event().0.clone()));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a function. Requires the [`UiRoot`] [`Resource`] to exist in the
    /// [`World`]. This method can be called repeatedly to register many such handlers.
    fn on_click_outside(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_outside_with_system(move |In((_, _))| handler())
    }

    /// On frames where this element is pressed or gets unpressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`] and its current pressed state. This method can be called repeatedly to register
    /// many such handlers.
    fn on_pressed_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .insert(PickingBehavior::default())
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    system_holder.set(Some(system));
                    observe(world, entity, move |press: Trigger<Press>, blocked: Query<&Blocked>, mut commands: Commands| {
                        let entity = press.entity();
                        if !blocked.contains(entity) {
                            commands.run_system_with_input(system, (entity, **press.event()));
                        }
                    });
                }))
                .on_event_with_system::<Pointer<Down>, _>(
                    move |In((entity, pointer_down)): In<(Entity, Pointer<Down>)>, world: &mut World| {
                        if matches!(pointer_down.button, PointerButton::Primary) {
                            if let Ok(mut entity) = world.get_entity_mut(entity) {
                                entity.insert(Pressable);
                            }
                        }
                    },
                )
                .apply(remove_system_holder_on_remove(system_holder))
        })
        // .on_global_event_with_system::<Pointer<Up>, _>(move |pointer_up: Listener<Pointer<Up>>| if
        // matches!(pointer_up.button, PointerButton::Primary) { down.set_neq(false) }) TODO: this isn't the
        // desired behavior, press should linger outside and instead `Up` should trigger even outside of element
        // (like the `.on_global_event_with_system` commented out above), requires being able to register
        // multiple event listeners per event type
        .on_hovered_change_with_system(|In((entity, hovered)): In<(_, bool)>, world: &mut World| {
            if !hovered {
                if let Ok(ref mut entity) = world.get_entity_mut(entity) {
                    EntityWorldMut::remove::<Pressable>(entity);
                }
            }
        })
    }

    /// When this element's pressed state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and its current pressed state. This method
    /// can be called repeatedly to register many such handlers.
    #[allow(clippy::type_complexity)]
    fn on_pressed_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Mutable::new(None);
        self.update_raw_el(clone!(
            (system_holder) | raw_el | {
                raw_el.on_spawn(clone!((system_holder) move |world, _| {
                    system_holder.set(Some(register_system(world, handler)));
                }))
            }
        ))
        .update_raw_el(remove_system_holder_on_remove(system_holder.clone()))
        .on_pressed_with_system_blockable::<_, PressHandlingBlocked>(
            move |In((entity, cur)),
                  mut pressed: Local<bool>,
                  mut system: Local<Option<SystemId<In<(Entity, bool)>>>>,
                  mut commands: Commands| {
                if cur != *pressed {
                    *pressed = cur;
                    // only pay the read locking cost once
                    let &mut system = system.get_or_insert_with(|| system_holder.get().unwrap());
                    commands.run_system_with_input(system, (entity, cur));
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
        handler: impl IntoSystem<In<Entity>, (), Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Mutable::new(None);
        self.update_raw_el(clone!(
            (system_holder) | raw_el | {
                raw_el
                    .on_spawn(clone!((system_holder) move |world, _| {
                        system_holder.set(Some(register_system(world, handler)));
                    }))
                    .apply(remove_system_holder_on_remove(system_holder.clone()))
            }
        ))
        .on_pressed_with_system_blockable::<_, Blocked>(
            move |In((entity, pressed)), mut system: Local<Option<SystemId<In<Entity>>>>, mut commands: Commands| {
                if pressed {
                    // only pay the read locking cost once
                    let &mut system = system.get_or_insert_with(|| system_holder.get().unwrap());
                    commands.run_system_with_input(system, entity);
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
        self.update_raw_el(|raw_el| raw_el.component_signal::<PressHandlingBlocked, _>(blocked.map_true(default)))
            .on_pressing_blockable::<PressHandlingBlocked>(handler)
    }

    /// When this element is being pressed, run a function.
    fn on_pressing(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_pressing_blockable::<PressHandlingBlocked>(handler)
    }

    /// When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`], waiting for the [`Future`] returned by `throttle` to complete
    /// before running the `handler` again.
    fn on_pressing_with_system_throttled<Fut: Future<Output = ()> + Send + 'static, Marker>(
        self,
        handler: impl IntoSystem<In<Entity>, (), Marker> + Send + 'static,
        mut throttle: impl FnMut() -> Fut + Send + 'static,
    ) -> Self {
        let (sender, receiver) = channel(());
        let system_holder = Mutable::new(None);
        self.update_raw_el(|raw_el| {
            raw_el
                .component_signal::<PressHandlingBlocked, _>(receiver.map_future(move |_| throttle()).map(|_| None))
                .observe(move |_: Trigger<OnAdd, PressHandlingBlocked>| sender.send(()).unwrap())
                .on_spawn(
                    clone!((system_holder) move |world, _| system_holder.set(Some(register_system(world, handler)))),
                )
        })
        .on_pressed_with_system_blockable::<_, PressHandlingBlocked>(
            move |In((entity, pressed)), mut system: Local<Option<SystemId<In<Entity>>>>, mut commands: Commands| {
                if pressed {
                    // only pay the read locking cost once
                    let &mut system = system.get_or_insert_with(|| system_holder.get().unwrap());
                    commands.run_system_with_input(system, entity);
                    if let Some(mut entity) = commands.get_entity(entity) {
                        entity.try_insert(PressHandlingBlocked);
                    }
                }
            },
        )
    }

    /// When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`], waiting for `duration` before running the `handler` again.
    fn on_pressing_with_system_with_sleep_throttle<Marker>(
        self,
        handler: impl IntoSystem<In<Entity>, (), Marker> + Send + 'static,
        duration: Duration,
    ) -> Self {
        self.on_pressing_with_system_throttled(handler, move || sleep(duration))
    }

    /// When this element is being pressed, run a function, waiting for the [`Future`] returned by
    /// `throttle` to complete before running the `handler` again.
    fn on_pressing_throttled<Fut: Future<Output = ()> + Send + 'static>(
        self,
        mut handler: impl FnMut() + Send + Sync + 'static,
        throttle: impl FnMut() -> Fut + Send + 'static,
    ) -> Self {
        self.on_pressing_with_system_throttled(move |_: In<_>| handler(), throttle)
    }

    /// When this element is being pressed, run a function, waiting for `duration` before running
    /// the `handler` again.
    fn on_pressing_with_sleep_throttle(
        self,
        handler: impl FnMut() + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        self.on_pressing_throttled(handler, move || sleep(duration))
    }

    /// Sync a [`Mutable`] with this element's pressed state.
    fn pressed_sync(self, pressed: Mutable<bool>) -> Self {
        self.on_pressed_change(move |cur| pressed.set_neq(cur))
    }
}

#[derive(Component, Deref, DerefMut)]
struct Hovered(bool);

#[derive(Component, Default)]
struct PressHandlingBlocked;

/// Fires when a the pointer crosses into the bounds of the `target` entity, ignoring children.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Enter {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a the pointer crosses out of the bounds of the `target` entity, excluding children.
#[derive(Clone, PartialEq, Debug, Reflect)]
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
    parent_query: Query<&Parent>,
    mut commands: Commands,
) {
    let pointer_id = PointerId::Mouse;
    let hover_set = hover_map.get(&pointer_id);
    for (entity, mut hovered) in hovereds.iter_mut() {
        let hit_data_option = match hover_set {
            Some(map) => map
                .iter()
                .find(|(ha, _)| **ha == entity || parent_query.iter_ancestors(**ha).any(|e| e == entity))
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
                commands.trigger_targets(Pointer::new(entity, pointer_id, location, Enter { hit }), entity);
            } else {
                // TODO: children `Leave`s don't trigger with this condition, e.g. in an aalo inspector row
                // if let Some(hit) = previous_hover_map
                // .get(&pointer_id)
                // .and_then(|map| map.get(&entity).cloned())
                // {
                // commands.trigger_targets(Pointer::new(pointer_id, location, entity, Leave { hit }), entity);
                commands.trigger_targets(Pointer::new(entity, pointer_id, location, Leave {}), entity);
                // }
            }
        }
    }
}

#[derive(Component)]
struct Pressable;

#[derive(Event, Deref)]
struct Press(bool);

#[allow(clippy::type_complexity)]
fn pressable_system(
    mut interaction_query: Query<(Entity, &PickingInteraction), (With<Pressable>, Changed<PickingInteraction>)>,
    mut commands: Commands,
) {
    for (entity, interaction) in &mut interaction_query {
        commands.trigger_targets(Press(matches!(interaction, PickingInteraction::Pressed)), entity);
    }
}

#[derive(Component)]
struct OnClickOutside;

#[derive(Event)]
struct ClickOutside(Pointer<Click>);

// TODO: use global events like moonzoon instead? requires being able to register multiple event
// listeners per event type (0.15)
fn on_click_outside(
    mut clicks: EventReader<Pointer<Click>>,
    on_click_outside_listeners: Query<Entity, With<OnClickOutside>>,
    children_query: Query<&Children>,
    ui_root: Res<UiRoot>,
    mut commands: Commands,
) {
    for click in clicks.read() {
        let entities = on_click_outside_listeners
            .iter()
            .filter(|&entity| !is_inside_or_removed_from_dom(entity, click, ui_root.0, &children_query));
        // TODO: avoid allocating entity vector
        commands.trigger_targets(ClickOutside(click.clone()), entities.collect::<Vec<_>>());
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
    if contains(element, event.target, children_query) {
        return true;
    }
    if !contains(ui_root, event.target, children_query) {
        return true;
    }
    false
}

#[derive(Component)]
struct CursorOver;

#[derive(Component, Default)]
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
        self.update_raw_el(|raw_el| {
            raw_el
                .insert((PickingBehavior::default(), CursorOverPropagationStopped))
                .observe(
                    |event: Trigger<OnInsert, CursorOver>,
                     cursor_on_hovers: Query<&CursorOnHover>,
                     disabled: Query<&Disabled>,
                     cursor_over_disabled_option: Option<Res<CursorOnHoverDisabled>>,
                     mut commands: Commands| {
                        let entity = event.entity();
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
                    |event: Trigger<OnInsert, CursorOnHover>,
                     cursor_overs: Query<&CursorOver>,
                     mut commands: Commands| {
                        let entity = event.entity();
                        if cursor_overs.contains(entity) {
                            if let Some(mut entity) = commands.get_entity(entity) {
                                entity.try_insert(CursorOver);
                            }
                        }
                    },
                )
                .insert(CursorOnHover(cursor_option))
                .observe(
                    move |event: Trigger<OnAdd, Disabled>,
                          cursor_over: Query<&CursorOver>,
                          pointer_map: Res<PointerMap>,
                          pointers: Query<&PointerLocation>,
                          hover_map: Res<HoverMap>,
                          mut pointer_over: EventWriter<Pointer<Over>>,
                          parents: Query<&Parent>,
                          mut commands: Commands| {
                        let entity = event.entity();
                        if let Some(mut entity) = commands.get_entity(entity) {
                            entity.remove::<CursorOverPropagationStopped>();
                        }
                        if cursor_over.get(entity).is_ok() {
                            if let Some(((hover_map, location), parent)) = hover_map
                                .get(&PointerId::Mouse)
                                .zip(
                                    pointer_map
                                        .get_entity(PointerId::Mouse)
                                        .and_then(|entity| pointers.get(entity).ok())
                                        .and_then(|pointer| pointer.location.clone()),
                                )
                                .zip(parents.get(entity).ok())
                            {
                                if let Some(hit) = hover_map.get(&entity).cloned() {
                                    pointer_over.send(Pointer::new(
                                        parent.get(),
                                        PointerId::Mouse,
                                        location,
                                        Over { hit },
                                    ));
                                }
                            }
                        }
                    },
                )
                .observe(
                    move |event: Trigger<OnRemove, Disabled>,
                          cursor_over: Query<&CursorOver>,
                          mut commands: Commands| {
                        let entity = event.entity();
                        if let Some(mut entity_commands) = commands.get_entity(entity) {
                            entity_commands.try_insert(CursorOverPropagationStopped);
                            if cursor_over.get(entity).is_ok() {
                                entity_commands.try_insert(CursorOver);
                            }
                        }
                    },
                )
                .on_event_with_system_propagation_stoppable::<Pointer<Over>, _, CursorOverPropagationStopped>(
                    |In((entity, _)), mut commands: Commands| {
                        if let Some(mut entity) = commands.get_entity(entity) {
                            entity.try_insert(CursorOver);
                        }
                    },
                )
                .on_event_with_system_stop_propagation::<Pointer<Out>, _>(|In((entity, _)), mut commands: Commands| {
                    if let Some(mut entity) = commands.get_entity(entity) {
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
        self.update_raw_el(|raw_el| {
            raw_el.component_signal::<CursorOnHover, _>(cursor_option_signal.map(Into::into).map(CursorOnHover))
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
        self.update_raw_el(|raw_el| raw_el.component_signal::<CursorDisabled, _>(disabled.map_true(default)))
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
        self.cursor_signal_disableable_signal(always(cursor_option.into()), disabled)
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
fn cursor_setter(
    event: Trigger<SetCursor>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if let Ok((entity, mut window)) = windows.get_single_mut() {
        let SetCursor(icon_option) = event.event();
        if let Some(icon) = icon_option.clone() {
            if let Some(mut window) = commands.get_entity(entity) {
                window.try_insert(icon);
            }
            window.cursor_options.visible = true;
        } else {
            window.cursor_options.visible = false;
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_event::<SetCursor>().add_observer(cursor_setter).add_systems(
        Update,
        (
            pressable_system.run_if(any_with_component::<Pressable>),
            update_hover_states.run_if(
                any_with_component::<Hovered>
                    // TODO: apparently this updates every frame no matter what, if so, remove this condition
                    // TODO: remove when native `Enter` and `Leave` available
                    .and(resource_exists_and_changed::<HoverMap>),
            ),
            consume_queued_cursor.run_if(resource_removed::<CursorOnHoverDisabled>),
            on_click_outside.run_if(
                resource_exists::<UiRoot>
                    .and(any_with_component::<OnClickOutside>)
                    .and(on_event::<Pointer<Click>>),
            ),
        ),
    );
}
