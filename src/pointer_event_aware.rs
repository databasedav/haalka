use std::{future::Future, ops::Not, time::Duration};

use apply::Apply;
use bevy::{
    ecs::{
        component::{ComponentHooks, StorageType},
        system::SystemId,
    },
    log::prelude::*,
    prelude::*,
    window::PrimaryWindow,
};
use bevy_mod_picking::{picking_core::backend::HitData, prelude::*};
use enclose::enclose as clone;
use focus::HoverMap;
use futures_signals::signal::{always, channel, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::SignalExtBool;

use crate::UiRoot;

use super::{
    raw::{observe, register_system, utils::remove_system_holder_on_remove, RawElWrapper},
    utils::sleep,
};

/// Enables reacting to pointer events like hover, click, and press. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon/tree/main)'s [`PointerEventAware`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/ability/pointer_event_aware.rs).
pub trait PointerEventAware: RawElWrapper {
    /// When this element's hovered state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this node's [`Entity`] and its current hovered state. This method
    /// can be called repeatedly to register many such handlers.
    fn on_hovered_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .insert(Pickable::default())
                .insert(Hovered(false))
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    system_holder.set(Some(system));
                    observe(world, entity, move |enter: Trigger<Pointer<Enter>>, mut commands: Commands| commands.run_system_with_input(system, (enter.entity(), true)));
                    observe(world, entity, move |leave: Trigger<Pointer<Leave>>, mut commands: Commands| commands.run_system_with_input(system, (leave.entity(), false)));
                }))
                .apply(remove_system_holder_on_remove(system_holder))
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
        handler: impl IntoSystem<(Entity, Pointer<Click>), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
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
                .insert(Pickable::default())
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

    // Requires the [`UiRoot`] resource to be present.
    fn on_click_outside_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, Pointer<Click>), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.with_entity(|mut entity| {
                let handler = entity.world_scope(|world| register_system(world, handler));
                entity.insert(OnClickOutside { handler });
            })
        })
    }

    // Requires the [`UiRoot`] resource to be present.
    fn on_click_outside(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_outside_with_system(move |In((_, _))| handler())
    }

    /// On frames where this element is pressed or gets unpressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this node's
    /// [`Entity`] and its current pressed state. This method can be called repeatedly to register
    /// many such handlers.
    fn on_pressed_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .insert(Pickable::default())
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
                            if let Some(mut entity) = world.get_entity_mut(entity) {
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
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    entity.remove::<Pressable>();
                }
            }
        })
    }

    /// When this element's pressed state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this node's [`Entity`] and its current pressed state. This method can
    /// be called repeatedly to register many such handlers.
    fn on_pressed_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
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
                  mut system: Local<Option<SystemId<(Entity, bool)>>>,
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
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this node's
    /// [`Entity`]. This method can be called repeatedly to register many such handlers.
    fn on_pressing_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<Entity, (), Marker> + Send + 'static,
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
            move |In((entity, pressed)), mut system: Local<Option<SystemId<Entity>>>, mut commands: Commands| {
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

    /// When this element is being pressed, run a function, waiting for the [`Future`] returned by
    /// `throttle` to resolve before running the `handler` again.
    fn on_pressing_throttled<Fut: Future<Output = ()> + Send + 'static>(
        self,
        mut handler: impl FnMut() + Send + Sync + 'static,
        mut throttle: impl FnMut() -> Fut + Send + 'static,
    ) -> Self {
        let (sender, receiver) = channel(());
        self.update_raw_el(|raw_el| {
            raw_el.component_signal::<PressHandlingBlocked, _>(receiver.map_future(move |_| throttle()).map(|_| None))
        })
        .on_pressed_with_system_blockable::<_, PressHandlingBlocked>(
            move |In((entity, pressed)), world: &mut World| {
                if pressed {
                    handler();
                    if let Some(mut entity) = world.get_entity_mut(entity) {
                        entity.insert(PressHandlingBlocked);
                        sender.send(()).unwrap();
                    }
                }
            },
        )
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
    hover_map: Res<bevy_mod_picking::focus::HoverMap>,
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
                commands.trigger_targets(Pointer::new(pointer_id, location, entity, Enter { hit }), entity);
            } else {
                // TODO: children `Leave`s don't trigger with this condition, e.g. in an aalo inspector row
                // if let Some(hit) = previous_hover_map
                // .get(&pointer_id)
                // .and_then(|map| map.get(&entity).cloned())
                // {
                // commands.trigger_targets(Pointer::new(pointer_id, location, entity, Leave { hit }), entity);
                commands.trigger_targets(Pointer::new(pointer_id, location, entity, Leave {}), entity);
                // }
            }
        }
    }
}

#[derive(Component)]
struct Pressable;

#[derive(Event, Deref)]
struct Press(bool);

fn pressable_system(
    mut interaction_query: Query<(Entity, &PickingInteraction), (With<Pressable>, Changed<PickingInteraction>)>,
    mut commands: Commands,
) {
    for (entity, interaction) in &mut interaction_query {
        commands.trigger_targets(Press(matches!(interaction, PickingInteraction::Pressed)), entity);
    }
}

struct OnClickOutside {
    handler: SystemId<(Entity, Pointer<Click>)>,
}

impl Component for OnClickOutside {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            if let Some(&Self { handler }) = world.get::<Self>(entity) {
                world.commands().add(move |world: &mut World| {
                    let _ = world.remove_system(handler);
                });
            }
        });
    }
}

// TODO: use global events like moonzoon instead? requires being able to register multiple event
// listeners per event type
fn on_click_outside_system(
    mut clicks: EventReader<Pointer<Click>>,
    on_click_outsides: Query<(Entity, &OnClickOutside)>,
    children_query: Query<&Children>,
    ui_root: Res<UiRoot>,
    mut commands: Commands,
) {
    for click in clicks.read() {
        for (entity, &OnClickOutside { handler }) in on_click_outsides.iter() {
            if !is_inside_or_removed_from_dom(entity, click, ui_root.0, &children_query) {
                commands.run_system_with_input(handler, (entity, click.clone()));
            }
        }
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
    let target = event.target();
    if contains(element, target, children_query) {
        return true;
    }
    if !contains(ui_root, target, children_query) {
        return true;
    }
    false
}

#[derive(Component)]
struct CursorOver;

#[derive(Component, Default)]
pub struct CursorDisabled;

#[derive(Resource)]
pub struct CursorOnHoverDisabled;

#[derive(Component)]
pub struct CursorOnHover(Option<CursorIcon>);

/// Enables reactively setting the cursor icon when an element receives an [`Over`] event.
pub trait CursorOnHoverable: PointerEventAware {
    /// Set the cursor icon when this element receives an [`Over`] event. When passed [`None`], the
    /// cursor is hidden.
    fn cursor(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        self.cursor_disableable::<CursorDisabled>(cursor_option)
    }

    /// on hover, the icon stored in the CursorOnHover component is set on the window
    fn cursor_disableable<DisabledComponent: Component>(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        let cursor_option = cursor_option.into();
        self.update_raw_el(|raw_el| {
            raw_el
                .insert((Pickable::default(), CursorOverPropagationStopped))
                .observe(
                    |event: Trigger<OnInsert, CursorOver>,
                     cursor_on_hovers: Query<&CursorOnHover>,
                     disabled: Query<&DisabledComponent>,
                     cursor_over_disabled_option: Option<Res<CursorOnHoverDisabled>>,
                     mut commands: Commands| {
                        let entity = event.entity();
                        if let Ok(&CursorOnHover(cursor_option)) = cursor_on_hovers.get(entity) {
                            if cursor_over_disabled_option.is_none() {
                                if disabled.contains(entity).not() {
                                    commands.trigger(CursorEvent(cursor_option));
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
                    move |event: Trigger<OnAdd, DisabledComponent>,
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
                                        PointerId::Mouse,
                                        location,
                                        parent.get(),
                                        Over { hit },
                                    ));
                                }
                            }
                        }
                    },
                )
                .observe(
                    move |event: Trigger<OnRemove, DisabledComponent>,
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

    /// Reactively set the cursor icon when this element receives an [`Over`] event, reactively
    /// disabling the cursor produced by this element. When the [`Signal`] outputs [`None`], the
    /// cursor is hidden.
    ///
    /// If the cursor is [`Over`] this element when it is reactively disabled, another [`Over`]
    /// event will be sent up the hierarchy to trigger any handlers whose propagation was previously
    /// stopped by this element.
    fn cursor_signal_disableable<DisabledComponent: Component>(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.component_signal::<CursorOnHover, _>(cursor_option_signal.map(Into::into).map(CursorOnHover))
        })
        .cursor_disableable::<DisabledComponent>(None)
    }

    /// Reactively set the cursor icon when this element receives an [`Over`] event, reactively
    /// disabling the cursor produced by this element. When the [`Signal`] outputs [`None`], the
    /// cursor is hidden.
    ///
    /// If the cursor is [`Over`] this element when it is reactively disabled, another [`Over`]
    /// event will be sent up the hierarchy to trigger any handlers whose propagation was previously
    /// stopped by this element.
    fn cursor_signal_disableable_signal(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.component_signal::<CursorDisabled, _>(disabled.map_true(default)))
            .cursor_signal_disableable::<CursorDisabled>(cursor_option_signal)
    }

    /// Reactively set the cursor icon when this element receives an [`Over`] event. When the
    /// [`Signal`] outputs [`None`], the cursor is hidden.
    fn cursor_signal<S: Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static>(
        mut self,
        cursor_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cursor_option_signal) = cursor_option_signal_option.into() {
            self = self.cursor_signal_disableable::<CursorDisabled>(cursor_option_signal);
        }
        self
    }

    // fn cursor_disableable<DisabledComponent: Component>(self, cursor_option: impl
    // Into<Option<CursorIcon>>) -> Self {
    //     self.cursor_signal_disableable::<DisabledComponent>(always(cursor_option.into()))
    // }

    /// Set the cursor icon when this element receives an [`Over`] event, reactively disabling the
    /// cursor produced by this element. When passed [`None`], the cursor is hidden.
    ///
    /// If the cursor is [`Over`] this element when it is
    /// reactively disabled, another [`Over`] event will be sent up the hierarchy to trigger any
    /// handlers whose propagation was previously stopped by this element.
    fn cursor_disableable_signal(
        self,
        cursor_option: impl Into<Option<CursorIcon>>,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.cursor_signal_disableable_signal(always(cursor_option.into()), disabled)
    }
}

#[derive(Event)]
pub struct CursorEvent(pub Option<CursorIcon>);

#[derive(Component)]
pub struct CursorOverPropagationStopped;

#[derive(Resource)]
struct QueuedCursor(Option<CursorIcon>);

fn consume_queued_cursor(queued_cursor: Option<Res<QueuedCursor>>, mut commands: Commands) {
    if let Some(cursor) = queued_cursor {
        commands.trigger(CursorEvent(cursor.0));
        commands.remove_resource::<QueuedCursor>();
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        EventListenerPlugin::<Pointer<Enter>>::default(),
        EventListenerPlugin::<Pointer<Leave>>::default(),
    ))
    .add_event::<CursorEvent>()
    // TODO: add support for multiple windows
    .observe(
        |event: Trigger<CursorEvent>, mut windows: Query<&mut Window, With<PrimaryWindow>>| {
            if let Ok(mut window) = windows.get_single_mut() {
                let CursorEvent(icon_option) = event.event();
                if let &Some(icon) = icon_option {
                    window.cursor.icon = icon;
                    window.cursor.visible = true;
                } else {
                    window.cursor.visible = false;
                }
            }
        },
    )
    .add_systems(
        Update,
        (
            pressable_system.run_if(any_with_component::<Pressable>),
            update_hover_states.run_if(
                any_with_component::<Hovered>
                    // TODO: apparently this updates every frame no matter what, if so, remove this condition
                    .and_then(resource_exists_and_changed::<bevy_mod_picking::focus::HoverMap>),
            ),
            consume_queued_cursor.run_if(resource_removed::<CursorOnHoverDisabled>()),
            on_click_outside_system.run_if(
                resource_exists::<UiRoot>
                    .and_then(any_with_component::<OnClickOutside>)
                    .and_then(on_event::<Pointer<Click>>()),
            ),
        ),
    );
}
