//! Semantics for managing global event listeners.

use std::sync::{Arc, OnceLock};

use super::{
    element::UiRoot,
    raw::{RawElWrapper, observe, register_system, utils::remove_system_holder_on_remove},
    utils::clone,
};
use apply::Apply;
use bevy_ecs::prelude::*;

/// Enables registering "global" event listeners on the [`UiRoot`] node. The [`UiRoot`] must be
/// manually registered with [`UiRootable::ui_root`](super::element::UiRootable::ui_root) for this
/// to work as expected.
pub trait GlobalEventAware: RawElWrapper {
    /// When an `E` [`Event`] propagates to the [`UiRoot`] node, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] (not the [`UiRoot`]'s) and the [`Event`].
    #[allow(clippy::type_complexity)]
    fn on_global_event_with_system<E: Event + Clone, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, E)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Arc::new(OnceLock::new());
            let observer_holder = Arc::new(OnceLock::new());
            raw_el
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
                .on_spawn_with_system(clone!((observer_holder, system_holder) move |In(entity), child_ofs: Query<&ChildOf>, ui_roots: Query<&UiRoot>, mut commands: Commands| {
                    for ancestor in child_ofs.iter_ancestors(entity) {
                        if ui_roots.contains(ancestor) {
                            commands.queue(clone!((system_holder, observer_holder) move |world: &mut World| {
                                let observer = observe(world, ancestor, clone!((system_holder) move |event: Trigger<E>, mut commands: Commands| {
                                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, (*event).clone()));
                                })).id();
                                let _ = observer_holder.set(observer);
                            }));
                            break;
                        }
                    }
                }))
                .on_remove(move |world, _| {
                    if let Some(&observer) = observer_holder.get() {
                        world.commands().queue(move |world: &mut World| {
                            let _ = world.try_despawn(observer);
                        })
                    }
                })
        })
    }

    /// When an `E` [`Event`] propagates to the [`UiRoot`] node, run a function with the [`Event`].
    fn on_global_event<E: Event + Clone>(self, mut handler: impl FnMut(E) + Send + Sync + 'static) -> Self {
        self.on_global_event_with_system::<E, _>(move |In((_, event))| handler(event))
    }
}
