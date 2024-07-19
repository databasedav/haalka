use super::{element::UiRoot, raw::RawElWrapper};
use bevy::prelude::*;
use bevy_eventlistener::prelude::*;

// TODO: there should be a way to pass the entity into the system
/// Enables registering "global" event listeners on the [`UiRoot`] node. The [`UiRoot`] must be
/// manually registered with [`UiRootable::ui_root`] for this to work as expected.
///
/// # Notes
/// Since multiple [`bevy_eventlistener::On`](bevy_eventlistener::event_listener::On)s can't be
/// registered on the same entity, this trait can't *yet* be used to do things like registering "on
/// click outside" listeners.
pub trait GlobalEventAware: RawElWrapper {
    /// When an `E` [`EntityEvent`] propagates to the [`UiRoot`] node, run a `handler` [`System`].
    fn on_global_event_with_system<E: EntityEvent, Marker>(
        self,
        handler: impl IntoSystem<(), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert_forwarded(ui_root_forwarder, On::<E>::run(handler)))
    }

    /// When an `E` [`EntityEvent`] propagates to the [`UiRoot`] node, run a function with access to
    /// the event's data.
    fn on_global_event<E: EntityEvent>(self, mut handler: impl FnMut(Listener<E>) + Send + Sync + 'static) -> Self {
        self.on_global_event_with_system::<E, _>(move |event: Listener<E>| handler(event))
    }

    /// When an `E` [`EntityEvent`] propagates to the [`UiRoot`] node, run a function with mutable
    /// access to the event's data.
    fn on_global_event_mut<E: EntityEvent>(
        self,
        mut handler: impl FnMut(ListenerMut<E>) + Send + Sync + 'static,
    ) -> Self {
        self.on_global_event_with_system::<E, _>(move |event: ListenerMut<E>| handler(event))
    }
}

fn ui_root_forwarder(entity: &mut EntityWorldMut) -> Option<Entity> {
    entity.world_scope(|world| world.get_resource::<UiRoot>().map(|&UiRoot(ui_root)| ui_root))
}
