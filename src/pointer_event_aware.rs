use std::time::Duration;

use bevy::{prelude::*, app::PluginGroupBuilder};
use bevy_eventlistener::{EventListenerPlugin, event_dispatcher::EventDispatcher, EventListenerSet};
use bevy_mod_picking::{prelude::*, picking_core::PickSet};
use enclose::enclose as clone;
use futures_signals::signal::{Mutable, SignalExt};

use crate::{RawElWrapper, spawn, sleep};

pub trait MouseInteractionAware: RawElWrapper {
    fn on_hovered_change(self, mut handler: impl FnMut(bool) + Clone + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
            .insert((
                Pickable::default(),
                On::<Pointer<Over>>::run(clone!((mut handler) move || handler(true))),
                On::<Pointer<Out>>::run(move || handler(false)),
            ))
        })
    }

    fn on_click(self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.insert((
                Pickable::default(),
                On::<Pointer<Click>>::run(handler),
            ))
        })
    }

    fn on_pressed_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
            .insert((
                Pickable::default(),
                Pressable(Box::new(move |is_pressed| handler(is_pressed))),
            ))
        })
    }

    fn on_pressing(self, mut handler: impl FnMut() + Clone + Send + Sync + 'static) -> Self {
        self.on_pressed_change(move |is_pressed| if is_pressed { handler() })
    }

    fn on_pressing_blockable(self, mut handler: impl FnMut() + Clone + Send + Sync + 'static, blocked: Mutable<bool>) -> Self {
        // TODO: should instead track pickability and just add/remove the Pressable on blocked change to minimize spurious handler calls, also blocked can then be a signal
        self.on_pressed_change(move |is_pressed| if is_pressed && !blocked.get() { handler() })
    }

    fn on_pressing_throttled(self, mut handler: impl FnMut() + Clone + Send + Sync + 'static, duration: Duration) -> Self {
        let blocked = Mutable::new(false);
        let throttler = spawn(clone!((blocked) async move {
            blocked.signal()
            .for_each(move |b| {
                clone!((blocked) async move {
                    if b {
                        sleep(duration).await;
                        blocked.set_neq(false);
                    }
                })
            })
            .await;
        }));
        self
        .update_raw_el(|raw_el| raw_el.hold_tasks([throttler]))
        .on_pressing_blockable(
            clone!((blocked) move || {
                handler();
                blocked.set_neq(true);
            }),
            blocked
        )
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
    mut interaction_query: Query<(&PickingInteraction, &mut Pressable), Changed<PickingInteraction>>  // TODO: does Changed actually do anything here ?
) {
    for (interaction, mut pressable) in &mut interaction_query {
        pressable.0(matches!(interaction, PickingInteraction::Pressed));
    }
}


#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PointerOverSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PointerOutSet;

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
                    .in_set(PointerOverSet)
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
                    .in_set(PointerOutSet)
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
            // so bubbling hover is always true last
            .configure_sets(PreUpdate, (PointerOutSet, PointerOverSet).chain())
            .add_plugins((
                PointerOverPlugin,
                PointerOutPlugin,
                EventListenerPlugin::<Pointer<Down>>::default(),
                EventListenerPlugin::<Pointer<Up>>::default(),
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
