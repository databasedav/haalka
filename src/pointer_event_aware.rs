use std::{fmt::Debug, future::Future, time::Duration};

use bevy::{app::PluginGroupBuilder, ecs::system::SystemId, prelude::*};
use bevy_eventlistener::{event_dispatcher::EventDispatcher, EventListenerPlugin, EventListenerSet};
use bevy_mod_picking::{picking_core::PickSet, prelude::*};
use enclose::enclose as clone;
use futures_signals::signal::{self, always, Mutable, Signal, SignalExt};
use futures_signals_ext::{SignalExtBool, SignalExtExt};

use crate::{sleep, spawn, RawElWrapper};

pub trait PointerEventAware: RawElWrapper {
    fn on_hovered_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .with_entity(move |mut entity| {
                    let system = entity.world_scope(|world| world.register_system(handler));
                    entity.insert(Hoverable {
                        system,
                        is_hovered: false,
                    });
                })
                // TODO: bevy 0.14, remove the system
                // .on_remove
                .insert(Pickable::default())
        })
    }

    fn on_hovered_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_hovered_change_with_system(move |In((_, is_hovered))| handler(is_hovered))
    }

    fn on_click_with_system<Marker>(self, handler: impl IntoSystem<(), (), Marker>) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event_with_system::<Pointer<Click>, _>(handler)
        })
    }

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

    fn on_click_event(self, mut handler: impl FnMut(&Pointer<Click>) + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event::<Pointer<Click>>(move |listener| handler(&*listener))
        })
    }

    fn on_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_event(move |click| {
            if matches!(click.button, PointerButton::Primary) {
                handler()
            }
        })
    }

    fn on_click_propagation_stoppable(
        self,
        mut handler: impl FnMut() + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.on_click_event_propagation_stoppable(move |_| handler(), propagation_stopped)
    }

    fn on_click_stop_propagation(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_propagation_stoppable(handler, always(true))
    }

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

    fn on_pressed_change_blockable_with_system<Marker>(
        self,
        handler: impl IntoSystem<(Entity, bool), (), Marker> + Send + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let down = Mutable::new(false);
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
                // TODO: up should trigger even outside of element
                .on_event_with_system::<Pointer<Up>, _>(move |pointer_up: Listener<Pointer<Up>>| if matches!(pointer_up.button, PointerButton::Primary) { down.set_neq(false) })
        })
    }

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
    fn on_pressed_change_blockable(
        self,
        mut handler: impl FnMut(bool) + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.on_pressed_change_blockable_with_system(move |In((_, is_pressed))| handler(is_pressed), blocked)
    }

    fn on_pressed_change(self, handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_pressed_change_blockable(handler, always(false))
    }

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

    fn on_pressing(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_pressing_blockable(handler, always(false))
    }

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

    fn on_pressing_with_sleep_throttle(
        self,
        handler: impl FnMut() + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        self.on_pressing_throttled(handler, move || sleep(duration))
    }

    fn hovered_sync(self, hovered: Mutable<bool>) -> Self {
        self.on_hovered_change(move |is_hovered| hovered.set_neq(is_hovered))
    }

    fn pressed_sync(self, pressed: Mutable<bool>) -> Self {
        self.on_pressed_change(move |is_pressed| pressed.set_neq(is_pressed))
    }
}

#[derive(Component)]
struct Hoverable {
    system: SystemId<(Entity, bool)>,
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
            commands.run_system_with_input(hoverable.system, (entity, is_hovering));
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

/// TODO: requires being able to register multipe callbacks for the same event type in event
/// listener, since otherwise would overwrite any other on hovered listeners
// trait Cursorable: PointerEventAware {
//     fn cursor(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
//     }
// }

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PointerOverSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PointerOutSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PointerDownSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PointerUpSet;

struct PointerOverPlugin;
impl Plugin for PointerOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Pointer<Over>>()
            .insert_resource(EventDispatcher::<Pointer<Over>>::default())
            .add_systems(
                PreUpdate,
                (
                    EventDispatcher::<Pointer<Over>>::build,
                    EventDispatcher::<Pointer<Over>>::bubble_events,
                    EventDispatcher::<Pointer<Over>>::cleanup,
                )
                    .chain()
                    .run_if(on_event::<Pointer<Over>>())
                    .in_set(EventListenerSet)
                    .in_set(PointerOverSet),
            );
    }
}

struct PointerOutPlugin;
impl Plugin for PointerOutPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Pointer<Out>>()
            .insert_resource(EventDispatcher::<Pointer<Out>>::default())
            .add_systems(
                PreUpdate,
                (
                    EventDispatcher::<Pointer<Out>>::build,
                    EventDispatcher::<Pointer<Out>>::bubble_events,
                    EventDispatcher::<Pointer<Out>>::cleanup,
                )
                    .chain()
                    .run_if(on_event::<Pointer<Out>>())
                    .in_set(EventListenerSet)
                    .in_set(PointerOutSet),
            );
    }
}

struct PointerDownPlugin;
impl Plugin for PointerDownPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Pointer<Down>>()
            .insert_resource(EventDispatcher::<Pointer<Down>>::default())
            .add_systems(
                PreUpdate,
                (
                    EventDispatcher::<Pointer<Down>>::build,
                    EventDispatcher::<Pointer<Down>>::bubble_events,
                    EventDispatcher::<Pointer<Down>>::cleanup,
                )
                    .chain()
                    .run_if(on_event::<Pointer<Down>>())
                    .in_set(EventListenerSet)
                    .in_set(PointerDownSet),
            );
    }
}

struct PointerUpPlugin;
impl Plugin for PointerUpPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Pointer<Up>>()
            .insert_resource(EventDispatcher::<Pointer<Up>>::default())
            .add_systems(
                PreUpdate,
                (
                    EventDispatcher::<Pointer<Up>>::build,
                    EventDispatcher::<Pointer<Up>>::bubble_events,
                    EventDispatcher::<Pointer<Up>>::cleanup,
                )
                    .chain()
                    .run_if(on_event::<Pointer<Up>>())
                    .in_set(EventListenerSet)
                    .in_set(PointerUpSet),
            );
    }
}

// TODO: don't need to manually order like this once mod picking has event ordering
struct RiggedInteractionPlugin;
impl Plugin for RiggedInteractionPlugin {
    fn build(&self, app: &mut App) {
        use events::*;
        use focus::{update_focus, update_interactions};

        app.init_resource::<focus::HoverMap>()
            .init_resource::<focus::PreviousHoverMap>()
            .init_resource::<DragMap>()
            .add_event::<PointerCancel>()
            .add_systems(
                PreUpdate,
                (
                    update_focus,
                    pointer_events,
                    update_interactions,
                    send_click_and_drag_events,
                    send_drag_over_events,
                )
                    .chain()
                    .in_set(PickSet::Focus),
            )
            // so exits always run last
            .configure_sets(
                PreUpdate,
                (PointerDownSet, PointerUpSet, PointerOutSet, PointerOverSet).chain(),
            )
            .add_plugins((
                PointerOverPlugin,
                PointerOutPlugin,
                PointerDownPlugin,
                PointerUpPlugin,
                EventListenerPlugin::<Pointer<Click>>::default(),
                EventListenerPlugin::<Pointer<Move>>::default(),
                EventListenerPlugin::<Pointer<DragStart>>::default(),
                EventListenerPlugin::<Pointer<Drag>>::default(),
                EventListenerPlugin::<Pointer<DragEnd>>::default(),
                EventListenerPlugin::<Pointer<DragEnter>>::default(),
                EventListenerPlugin::<Pointer<DragOver>>::default(),
                EventListenerPlugin::<Pointer<DragLeave>>::default(),
                EventListenerPlugin::<Pointer<Drop>>::default(),
            ));
    }
}

pub(crate) struct RiggedPickingPlugin;
impl PluginGroup for RiggedPickingPlugin {
    fn build(self) -> PluginGroupBuilder {
        let mut builder = PluginGroupBuilder::start::<Self>();

        builder = builder
            .add(picking_core::CorePlugin)
            .add(RiggedInteractionPlugin)
            .add(input::InputPlugin)
            .add(BevyUiBackend);

        builder
    }
}

pub(crate) struct PointerEventAwarePlugin;
impl Plugin for PointerEventAwarePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                pressable_system.run_if(any_with_component::<Pressable>),
                update_hover_states.run_if(
                    any_with_component::<Hoverable>
                        .and_then(resource_exists_and_changed::<bevy_mod_picking::focus::HoverMap>),
                ),
            ),
        );
    }
}
