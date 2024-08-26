use std::{future::Future, time::Duration};

use bevy::{ecs::system::SystemId, log::prelude::*, prelude::*, window::PrimaryWindow};
use bevy_mod_picking::{picking_core::backend::HitData, prelude::*};
use enclose::enclose as clone;
use focus::{HoverMap, PreviousHoverMap};
use futures_signals::signal::{always, channel, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::{SignalExtBool, SignalExtExt};

use super::{
    raw::{observe, register_system, RawElWrapper},
    utils::sleep,
};

/// Enables reacting to pointer events like hover, click, and press. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon/tree/main)'s [`PointerEventAware`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/ability/pointer_event_aware.rs).
pub trait PointerEventAware: RawElWrapper {
    /// When this element's hovered state changes, run a system which takes [`In`](`System::In`)
    /// this node's [`Entity`] and its current hovered state.
    fn on_hovered_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_id_holder = Mutable::new(None);
            raw_el
                .insert(Pickable::default())
                .insert(Hovered(false))
                .on_spawn(clone!((system_id_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    system_id_holder.set(Some(system));
                    observe(world, entity, move |enter: Trigger<Pointer<Enter>>, mut commands: Commands| commands.run_system_with_input(system, (enter.entity(), true)));
                    observe(world, entity, move |leave: Trigger<Pointer<Leave>>, mut commands: Commands| commands.run_system_with_input(system, (leave.entity(), false)));
                }))
                .on_remove(move |world, _| {
                    if let Some(system) = system_id_holder.get() {
                        world.commands().add(move |world: &mut World| {
                            let _ = world.remove_system(system);
                        })
                    }
                })
        })
    }

    /// When this element's hover state changes, run a function with its current hovered state.
    fn on_hovered_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_hovered_change_with_system(move |In((_, is_hovered))| handler(is_hovered))
    }

    /// Sync a [`Mutable`] with this element's hovered state.
    fn hovered_sync(self, hovered: Mutable<bool>) -> Self {
        self.on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    }

    /// Run a system when this element is clicked.
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

    /// Run a function when this element is clicked.
    fn on_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_with_system(move |In((_, click)): In<(_, Pointer<Click>)>| {
            if matches!(click.button, PointerButton::Primary) {
                handler()
            }
        })
    }

    /// Run a function when this element is clicked, reactively controlling whether the click
    /// bubbles up the hierarchy.
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

    /// Run a function when this element is clicked, stopping the click from bubbling up the
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

    // TODO: this doesn't make sense until the event listener supports registering multiple listeners https://discord.com/channels/691052431525675048/1236111180624297984/1250245547756093465
    // per event
    // fn on_click_outside_event(self, mut handler: impl FnMut(&Pointer<Click>) + Send + Sync + 'static)
    // -> Self {      self.update_raw_el(|raw_el| {
    //         let entity_holder = Mutable::new(None);
    //         raw_el
    //             .on_spawn(clone!((entity_holder) move |_, entity| entity_holder.set(Some(entity))))
    //             .on_global_event_with_system::<Pointer<Click>, _>(
    //                 move |click: Listener<Pointer<Click>>, children_query: Query<&Children>| {
    //                     if !is_inside_or_removed_from_dom(
    //                         entity_holder.get().unwrap(),
    //                         &click,
    //                         click.listener(),
    //                         &children_query,
    //                     ) {
    //                         handler(&*click);
    //                     }
    //                 },
    //             )
    //     })
    // }

    // fn on_click_outside(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
    //     self.on_click_outside_event(move |_| handler())
    // }

    /// On frames where this element is pressed or gets unpressed and the `blocked` [`System`]
    /// returns `false`, run a `handler` [`System`]; the `blocked` [`System`] takes
    /// [`In`](`System::In`) this node's [`Entity`] and the `handler` [`System`] takes
    /// [`In`](`System::In`) this node's [`Entity`] and its current pressed state.
    fn on_pressed_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .insert(Pickable::default())
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let handler = register_system(world, handler);
                    system_holder.set(Some(handler));
                    observe(world, entity, move |press: Trigger<Press>, blocked: Query<&Blocked>, mut commands: Commands| {
                        let entity = press.entity();
                        if !blocked.contains(entity) {
                            commands.run_system_with_input(handler, (entity, **press.event()));
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
                .on_remove(move |world, _| {
                    if let Some(handler) = system_holder.get() {
                        world.commands().add(move |world: &mut World| {
                            let _ = world.remove_system(handler);
                        })
                    }
                })
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

    fn on_pressed_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Mutable::new(None);
        self.update_raw_el(|raw_el| {
            raw_el.on_spawn(clone!((system_holder) move |world, _| {
                system_holder.set(Some(register_system(world, handler)));
            }))
        })
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

    fn on_pressed_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_pressed_change_with_system(move |In((_, pressed))| handler(pressed))
    }

    fn on_pressing_with_system_blockable<Marker, Blocked: Component>(
        self,
        handler: impl IntoSystem<Entity, (), Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Mutable::new(None);
        self.update_raw_el(|raw_el| {
            raw_el.on_spawn(clone!((system_holder) move |world, _| {
                system_holder.set(Some(register_system(world, handler)));
            }))
        })
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

    fn on_pressing_blockable<Blocked: Component>(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_pressing_with_system_blockable::<_, Blocked>(move |_: In<_>| handler())
    }

    /// When this element is being pressed, run a function, reactively controlling whether the press
    /// is blocked.
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
    previous_hover_map: Res<PreviousHoverMap>,
    mut hovereds: Query<(Entity, &mut Hovered)>,
    parent_query: Query<&Parent>,
    // mut pointer_enter: EventWriter<Pointer<Enter>>,
    // mut pointer_leave: EventWriter<Pointer<Leave>>,
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

// TODO: used in on_click_outside_event, but requires being able to register multiple handlers per event https://discord.com/channels/691052431525675048/1236111180624297984/1250245547756093465
// fn contains(left: Entity, right: Entity, children_query: &Query<&Children>) -> bool {
//     children_query.iter_descendants(left).any(|e| e == right)
// }

// // TODO: add support for some sort of exclusion
// fn is_inside_or_removed_from_dom(
//     element: Entity,
//     event: &Listener<Pointer<Click>>,
//     ui_root: Entity,
//     children_query: &Query<&Children>,
// ) -> bool {
//     let target = event.target();
//     if contains(element, target, children_query) {
//         return true;
//     }
//     if !contains(ui_root, target, children_query) {
//         return true;
//     }
//     false
// }

#[derive(Component)]
struct CursorOver;

#[derive(Component, Default)]
pub struct CursorDisabled;

#[derive(Resource)]
pub struct CursorOverDisabled;

/// Enables reactively setting the cursor icon when an element receives an [`Over`] event.
pub trait CursorOnHoverable: PointerEventAware {
    /// Set the cursor icon when this element receives an [`Over`] event. When passed [`None`], the
    /// cursor is hidden.
    fn cursor(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        let cursor_option = cursor_option.into();
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event_with_system_stop_propagation::<Pointer<Over>, _>(
                    move |_: In<_>,
                          cursor_over_disabled_option: Option<Res<CursorOverDisabled>>,
                          mut commands: Commands| {
                        if cursor_over_disabled_option.is_none() {
                            commands.trigger(CursorEvent(cursor_option));
                        }
                    },
                )
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
        mut self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>>> + Send + Sync + 'static,
    ) -> Self {
        if let Some(cursor_option_signal) = cursor_option_signal.into() {
            let over = Mutable::new(false);
            let cursor_option_signal = cursor_option_signal
                .map(|cursor_option| cursor_option.into())
                .broadcast();
            self = self.update_raw_el(move |raw_el| {
                raw_el
                    .insert((Pickable::default(), CursorOverPropagationStopped))
                    .on_signal_one_shot(
                        over.signal().map_true_signal(move || cursor_option_signal.signal()),
                        |In((entity, cursor_option_option)): In<(Entity, Option<Option<CursorIcon>>)>,
                         disabled: Query<&DisabledComponent>,
                         cursor_over_disabled_option: Option<Res<CursorOverDisabled>>,
                         mut commands: Commands| {
                            if cursor_over_disabled_option.is_none() {
                                if let Some(cursor_option) = cursor_option_option {
                                    if !disabled.contains(entity) {
                                        println!("{:?}", cursor_option);
                                        commands.trigger(CursorEvent(cursor_option));
                                    }
                                }
                            }
                        },
                    )
                    .on_event_with_system_propagation_stoppable::<Pointer<Over>, _, CursorOverPropagationStopped>(
                        |In((entity, _)), world: &mut World| {
                            if let Some(mut entity) = world.get_entity_mut(entity) {
                                if entity.get::<DisabledComponent>().is_none() {
                                    entity.insert(CursorOver);
                                }
                            }
                        },
                    )
                    .on_event_with_system_stop_propagation::<Pointer<Out>, _>(|In((entity, _)), world: &mut World| {
                        if let Some(mut entity) = world.get_entity_mut(entity) {
                            entity.remove::<CursorOver>();
                        }
                    })
                    .observe(|event: Trigger<OnInsert, DisabledComponent>, mut commands: Commands| {
                        if let Some(mut entity) = commands.get_entity(event.entity()) {
                            entity.remove::<CursorOverPropagationStopped>();
                        }
                    })
                    .observe(
                        clone!((over) move |event: Trigger<OnAdd, DisabledComponent>, mut commands: Commands| {
                            if let Some(mut entity) = commands.get_entity(event.entity()) {
                                entity.insert(CursorOverPropagationStopped);
                            }
                            over.set(true);
                        }),
                    )
                    .observe(
                        clone!((over) move |event: Trigger<OnInsert, CursorOver>, disabled: Query<&DisabledComponent>| {
                            if !disabled.contains(event.entity()) {
                                over.set(true);
                            }
                        }),
                    )
                    .observe(clone!((over) move |_: Trigger<OnRemove, CursorOver>| over.set_neq(false)))
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
                            if cursor_over.get(entity).is_ok() {
                                commands.entity(entity).remove::<CursorOver>();
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
                              pointer_map: Res<PointerMap>,
                              pointers: Query<&PointerLocation>,
                              hover_map: Res<HoverMap>,
                              mut pointer_over: EventWriter<Pointer<Over>>,
                              mut commands: Commands| {
                            let entity = event.entity();
                            if cursor_over.get(entity).is_ok() {
                                commands.entity(entity).remove::<CursorOver>();
                                if let Some((hover_map, location)) = hover_map.get(&PointerId::Mouse).zip(
                                    pointer_map
                                        .get_entity(PointerId::Mouse)
                                        .and_then(|entity| pointers.get(entity).ok())
                                        .and_then(|pointer| pointer.location.clone()),
                                ) {
                                    if let Some(hit) = hover_map.get(&entity).cloned() {
                                        pointer_over.send(Pointer::new(
                                            PointerId::Mouse,
                                            location,
                                            entity,
                                            Over { hit },
                                        ));
                                    }
                                }
                            }
                        },
                    )
            });
        }
        self
    }

    /// Reactively set the cursor icon when this element receives an [`Over`] event, reactively
    /// disabling the cursor produced by this element. When the [`Signal`] outputs [`None`], the
    /// cursor is hidden.
    ///
    /// If the cursor is [`Over`] this element when it is reactively disabled, another [`Over`]
    /// event will be sent up the hierarchy to trigger any handlers whose propagation was previously
    /// stopped by this element.
    fn cursor_signal_disableable_signal<DisabledComponent: Component + Default>(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>>> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.component_signal::<DisabledComponent, _>(disabled.map_true(default)))
            .cursor_signal_disableable::<DisabledComponent>(cursor_option_signal)
    }

    /// Reactively set the cursor icon when this element receives an [`Over`] event. When the
    /// [`Signal`] outputs [`None`], the cursor is hidden.
    fn cursor_signal<S: Signal<Item = impl Into<Option<CursorIcon>>> + Send + Sync + 'static>(
        mut self,
        cursor_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cursor_option_signal) = cursor_option_signal_option.into() {
            self = self.cursor_signal_disableable::<CursorDisabled>(cursor_option_signal);
        }
        self
    }

    fn cursor_disableable<DisabledComponent: Component>(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        self.cursor_signal_disableable::<DisabledComponent>(always(cursor_option.into()))
    }

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
        self.cursor_signal_disableable_signal::<CursorDisabled>(always(cursor_option.into()), disabled)
    }
}

#[derive(Event)]
pub struct CursorEvent(pub Option<CursorIcon>);

#[derive(Component)]
pub struct CursorOverPropagationStopped;

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
                if let Some(icon) = icon_option {
                    window.cursor.icon = *icon;
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
                    // TODO: apparently this updates every no matter what, if so, remove this condition
                    .and_then(resource_exists_and_changed::<bevy_mod_picking::focus::HoverMap>),
            ),
        ),
    );
}
