//! Semantics for managing global event listeners.

use super::{
    element::UiRoot,
    raw::{observe, register_system, utils::remove_system_holder_on_remove, RawElWrapper},
    utils::clone,
};
use apply::Apply;
use bevy_ecs::{prelude::*, system::SystemId};
use futures_signals::signal::Mutable;

// TODO: there should be a way to pass the entity into the system
// TODO: 0.15
/// Enables registering "global" event listeners on the [`UiRoot`] node. The [`UiRoot`] must be
/// manually registered with [`UiRootable::ui_root`](super::element::UiRootable::ui_root) for this
/// to work as expected.
///
/// # Notes
/// Since multiple [`bevy_eventlistener::On`](bevy_eventlistener::event_listener::On)s can't be
/// registered on the same entity, this trait can't *yet* be used to do things like registering "on
/// click outside" listeners.
pub trait GlobalEventAware: RawElWrapper {
    /// When an `E` [`Event`] propagates to the [`UiRoot`] node, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] (not the [`UiRoot`]'s) and the [`Event`].
    #[allow(clippy::type_complexity)]
    fn on_global_event_with_system<E: Event + Clone, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            let observer_holder = Mutable::new(None);
            raw_el
                .on_spawn(clone!((system_holder) move |world, _| {
                    system_holder.set(Some(register_system(world, handler)));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
                .on_spawn(clone!((observer_holder) move |world, entity| {
                    if let Some(ui_root) = world.get_resource::<UiRoot>().map(|&UiRoot(ui_root)| ui_root) {
                        let observer = observe(world, ui_root, move |event: Trigger<E>, mut system: Local<Option<SystemId<In<(Entity, E)>>>>, mut commands: Commands| {
                            // only pay the read locking cost once
                            let &mut system = system.get_or_insert_with(|| system_holder.get().unwrap());
                            commands.run_system_with_input(system, (entity, (*event).clone()));
                        }).id();
                        observer_holder.set(Some(observer));
                    }
                }))
                .on_remove(move |world, _| {
                    if let Some(observer) = observer_holder.get() {
                        world.commands().queue(move |world: &mut World| {
                            world.despawn(observer);
                        })
                    }
                })
        })
    }

    /// When an `E` [`Event`] propagates to the [`UiRoot`] node, run a function with access to
    /// the event's data.
    fn on_global_event<E: Event + Clone>(self, mut handler: impl FnMut(E) + Send + Sync + 'static) -> Self {
        self.on_global_event_with_system::<E, _>(move |In((_, event))| handler(event))
    }
}
