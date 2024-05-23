use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_eventlistener::{event_dispatcher::EventDispatcher, EventListenerPlugin, EventListenerSet};
use bevy_mod_picking::{picking_core::PickSet, prelude::*};
use enclose::enclose as clone;
use futures_signals::signal::{self, always, Mutable, Signal, SignalExt};
use futures_signals_ext::{SignalExtBool, SignalExtExt};

use crate::{sleep, spawn, RawElWrapper};

pub trait PointerEventAware: RawElWrapper {
    fn on_hovered_change(self, handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        let handler = Arc::new(Mutex::new(handler));
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event::<Pointer<Over>>(clone!((mut handler) move || handler.lock().unwrap()(true)))
                .on_event::<Pointer<Out>>(move || handler.lock().unwrap()(false))
        })
    }

    fn on_click_with_system<Marker>(self, handler: impl IntoSystem<(), (), Marker>) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event_with_system::<Pointer<Click>, _>(handler)
        })
    }

    fn on_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_with_system(move |click: Listener<Pointer<Click>>| {
            if matches!(click.button, PointerButton::Primary) {
                handler()
            }
        })
    }

    fn on_click_propagation_stoppable(
        self,
        handler: impl FnMut() + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .insert(Pickable::default())
                .on_event_propagation_stoppable::<Pointer<Click>>(handler, propagation_stopped)
        })
    }

    fn on_click_stop_propagation(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_propagation_stoppable(handler, always(true))
    }

    fn on_right_click(self, mut handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click_with_system(move |click: Listener<Pointer<Click>>| {
            if matches!(click.button, PointerButton::Secondary) {
                handler()
            }
        })
    }

    // TODO: there's still problems with this, `Up` doesn't trigger outside of the element + pressable
    // system isn't sensitive to left clicks only, so e.g. downing a button, upping outside it, then
    // holding right click over it will incorrectly show a pressed state
    // TODO: add right click pressing convenience methods if someone wants them ...
    fn on_pressed_change_blockable(
        self,
        handler: impl FnMut(bool) + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let down = Mutable::new(false);
            let handler = Arc::new(Mutex::new(handler));
            raw_el
                .component_signal::<Pressable>(signal::and(signal::not(blocked), down.signal()).map_true(move || {
                    Pressable(Box::new(clone!((handler) move |is_pressed|
                        (handler.lock().unwrap())(is_pressed))))
                }))
                .insert(Pickable::default())
                .on_event_with_system::<Pointer<Down>, _>(clone!((down) move |pointer_down: Listener<Pointer<Down>>| if matches!(pointer_down.button, PointerButton::Primary) { down.set_neq(true) }))
                .on_event_with_system::<Pointer<Up>, _>(move |pointer_up: Listener<Pointer<Up>>| if matches!(pointer_up.button, PointerButton::Primary) { down.set_neq(false) })
        })
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
pub(crate) struct Pressable(Box<dyn FnMut(bool) + Send + Sync + 'static>);

pub(crate) fn pressable_system(
    mut interaction_query: Query<(&PickingInteraction, &mut Pressable), Changed<PickingInteraction>>,
) {
    for (interaction, mut pressable) in &mut interaction_query {
        pressable.0(matches!(interaction, PickingInteraction::Pressed));
    }
}

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
        app.add_systems(Update, pressable_system.run_if(any_with_component::<Pressable>));
    }
}
