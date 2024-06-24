use std::{future::Future, time::Duration};

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_mod_picking::prelude::*;
use enclose::enclose as clone;
use futures_signals::signal::{self, always, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::{SignalExtBool, SignalExtExt};

use super::{
    raw::RawElWrapper,
    utils::{sleep, spawn},
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
            raw_el
                .with_entity(move |mut entity| {
                    let system = entity.world_scope(|world| world.register_system(handler));
                    if let Some(mut hoverable) = entity.get_mut::<Hoverable>() {
                        hoverable.systems.push(system);
                    } else {
                        entity.insert(Hoverable {
                            systems: vec![system],
                            is_hovered: false,
                        });
                    }
                })
                // TODO: bevy 0.14, remove the system
                // .on_remove
                .insert(Pickable::default())
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
    fn on_click_with_system<Marker>(self, handler: impl IntoSystem<(), (), Marker>) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event_with_system::<Pointer<Click>, _>(handler)
        })
    }

    /// When this element is clicked, run a function with access to the click event, reactively
    /// controlling whether the click bubbles up the hierarchy.
    fn on_click_event_propagation_stoppable(
        self,
        handler: impl FnMut(&Pointer<Click>) + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event_propagation_stoppable::<Pointer<Click>>(handler, propagation_stopped)
        })
    }

    /// When this element is clicked, run a function with access to the click event.
    fn on_click_event(self, mut handler: impl FnMut(&Pointer<Click>) + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event::<Pointer<Click>>(move |listener| handler(&*listener))
        })
    }

    /// Run a function when this element is clicked.
    fn on_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_event(move |click| {
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
        self.on_click_event_propagation_stoppable(move |_| handler(), propagation_stopped)
    }

    /// Run a function when this element is clicked, stopping the click from bubbling up the
    /// hierarchy.
    fn on_click_stop_propagation(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_propagation_stoppable(handler, always(true))
    }

    /// Run a function when this element is right clicked.
    fn on_right_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_event(move |click| {
            if matches!(click.button, PointerButton::Secondary) {
                handler()
            }
        })
    }

    // TODO: this doesn't make sense until the event listener supports registering multiple listeners https://discord.com/channels/691052431525675048/1236111180624297984/1250245547756093465
    // per event fn on_click_outside_event(self, mut handler: impl FnMut(&Pointer<Click>) + Send +
    // Sync + 'static) -> Self {     self.update_raw_el(|raw_el| {
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

    /// When this element's pressed state changes, run a system which takes [`In`](`System::In`)
    /// this node's [`Entity`] and its current pressed state, reactively controlling whether the
    /// press is blocked.
    fn on_pressed_change_blockable_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        let down = Mutable::new(false);
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el
                .on_spawn(clone!((system_holder) move |world, _| {
                    let system = world.register_system(handler);
                    system_holder.set(Some(system));
                }))
                .component_signal::<Pressable, _>(
                    signal::and(system_holder.signal_ref(Option::is_some), signal::and(signal::not(blocked), down.signal())).dedupe().map_true(move || {
                    Pressable(system_holder.get().unwrap())
                }))
                .insert(Pickable::default())
                .on_event_with_system::<Pointer<Down>, _>(clone!((down) move |pointer_down: Listener<Pointer<Down>>| if matches!(pointer_down.button, PointerButton::Primary) { down.set_neq(true) }))
                // .on_global_event_with_system::<Pointer<Up>, _>(move |pointer_up: Listener<Pointer<Up>>| if matches!(pointer_up.button, PointerButton::Primary) { down.set_neq(false) })
            })
        // TODO: this isn't the desired behavior, press should linger outside and instead `Up` should trigger even outside of element (like the `.on_global_event_with_system` commented out above), requires being able to register multiple event listeners per event type
        .on_hovered_change(move |is_hovered| if !is_hovered { down.set_neq(false) })
    }

    /// When this element's pressed state changes, run a system which takes [`In`](`System::In`)
    /// this node's [`Entity`] and its current pressed state.
    fn on_pressed_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        self.on_pressed_change_blockable_with_system(handler, always(false))
    }

    // TODO: there's still problems with this, `Up` doesn't trigger outside of the element + pressable
    // system isn't sensitive to left clicks only, so e.g. downing a button, upping outside it, then
    // holding right click over it will incorrectly show a pressed state
    // TODO: add right click pressing convenience methods if someone wants them ...
    /// When this element's pressed state changes, run a function with its current pressed state,
    /// reactively controlling whether the press is blocked.
    fn on_pressed_change_blockable(
        self,
        mut handler: impl FnMut(bool) + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.on_pressed_change_blockable_with_system(move |In((_, is_pressed))| handler(is_pressed), blocked)
    }

    /// When this element's pressed state changes, run a function with its current pressed state.
    fn on_pressed_change(self, handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_pressed_change_blockable(handler, always(false))
    }

    /// When this element is being pressed, run a function, reactively controlling whether the press
    /// is blocked.
    fn on_pressing_blockable(
        self,
        mut handler: impl FnMut() + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.on_pressed_change_blockable(
            move |is_pressed| {
                if is_pressed {
                    handler()
                }
            },
            blocked,
        )
    }

    /// When this element is being pressed, run a function.
    fn on_pressing(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_pressing_blockable(handler, always(false))
    }

    /// When this element is being pressed, run a function, waiting for the [`Future`] returned by
    /// `throttle` to resolve before running the `handler` again.
    fn on_pressing_throttled<Fut: Future<Output = ()> + Send>(
        self,
        mut handler: impl FnMut() + Send + Sync + 'static,
        throttle: impl FnMut() -> Fut + Send + 'static,
    ) -> Self {
        let blocked = Mutable::new(false);
        let throttler = spawn(clone!((blocked) async move {
            blocked.signal()
            .throttle(throttle)
            .for_each_sync(move |b| {
                if b {
                    blocked.set_neq(false);
                }
            })
            .await;
        }));
        self.update_raw_el(|raw_el| raw_el.hold_tasks([throttler]))
            .on_pressing_blockable(
                clone!((blocked) move || {
                    handler();
                    blocked.set_neq(true);
                }),
                blocked.signal(),
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
        self.on_pressed_change(move |is_pressed| pressed.set_neq(is_pressed))
    }
}

#[derive(Component)]
struct Hoverable {
    systems: Vec<SystemId<(Entity, bool)>>,
    is_hovered: bool,
}

fn update_hover_states(
    hover_map: Res<bevy_mod_picking::focus::HoverMap>,
    mut hovers: Query<(Entity, &mut Hoverable)>,
    parent_query: Query<&Parent>,
    mut commands: Commands,
) {
    let hover_set = hover_map.get(&PointerId::Mouse);
    for (entity, mut hoverable) in hovers.iter_mut() {
        let is_hovering = match hover_set {
            Some(map) => map
                .iter()
                .any(|(ha, _)| *ha == entity || parent_query.iter_ancestors(*ha).any(|e| e == entity)),
            None => false,
        };
        if hoverable.is_hovered != is_hovering {
            hoverable.is_hovered = is_hovering;
            for &system in hoverable.systems.iter() {
                commands.run_system_with_input(system, (entity, is_hovering));
            }
        }
    }
}

#[derive(Component)]
struct Pressable(SystemId<(Entity, bool)>);

fn pressable_system(
    mut interaction_query: Query<(Entity, &PickingInteraction, &Pressable), Changed<PickingInteraction>>,
    mut commands: Commands,
) {
    for (entity, interaction, pressable) in &mut interaction_query {
        commands.run_system_with_input(
            pressable.0,
            (entity, matches!(interaction, PickingInteraction::Pressed)),
        );
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

// TODO: requires being able to register multipe callbacks for the same event type in event
// listener, since otherwise would overwrite any other on hovered listeners TODO: should be able to
// do this just with an Over with stop propagation

// trait Cursorable: PointerEventAware {
//     fn cursor(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
//     }
// }

pub(crate) struct PointerEventAwarePlugin;

impl Plugin for PointerEventAwarePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                pressable_system.run_if(any_with_component::<Pressable>),
                update_hover_states.run_if(
                    any_with_component::<Hoverable>
                        // TODO: apparently this updates every no matter what, if so, remove this condition
                        .and_then(resource_exists_and_changed::<bevy_mod_picking::focus::HoverMap>),
                ),
            ),
        );
    }
}
