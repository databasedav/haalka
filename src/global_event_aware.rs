//! Semantics for managing global event listeners.

use bevy_platform::sync::{Arc, OnceLock};

use super::{
    element::{BuilderWrapper, UiRoot},
    utils::{clone, observe, register_system, remove_system_holder_on_despawn},
};
use apply::Apply;
use bevy_ecs::{event::PropagateEntityTrigger, prelude::*, traversal::Traversal};
use jonmo::utils::LazyEntity;

/// Enables registering "global" event listeners on the [`UiRoot`] node. The [`UiRoot`] must be
/// manually registered with [`UiRootable::ui_root`](super::element::UiRootable::ui_root) for this
/// to work as expected.
pub trait GlobalEventAware: BuilderWrapper {
    /// When an `E` [`Event`] propagates to the [`UiRoot`] node, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] (not the [`UiRoot`]'s) and a
    /// [`GlobalEventData`] with the [`Event`].
    #[allow(clippy::type_complexity)]
    fn on_global_event<E, const AUTO_PROPAGATE: bool, T, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, GlobalEventData<E>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self
    where
        E: EntityEvent + for<'a> Event<Trigger<'a> = PropagateEntityTrigger<AUTO_PROPAGATE, E, T>> + Clone,
        T: Traversal<E> + 'static,
    {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            let observer_holder = LazyEntity::new();
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_despawn(system_holder.clone()))
                .on_spawn_with_system(clone!((observer_holder, system_holder) move |In(entity), child_ofs: Query<&ChildOf>, ui_roots: Query<&UiRoot>, mut commands: Commands| {
                    for ancestor in child_ofs.iter_ancestors(entity) {
                        if ui_roots.contains(ancestor) {
                            commands.queue(clone!((system_holder, observer_holder) move |world: &mut World| {
                                let observer = observe(world, ancestor, clone!((system_holder) move |event: On<E>, mut commands: Commands| {
                                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, GlobalEventData { original_event_target: event.original_event_target(), event: event.clone() }));
                                })).id();
                                observer_holder.set(observer);
                            }));
                            return;
                        }
                    }
                    if cfg!(debug_assertions) {
                        panic!("element must be a descendent of a `UiRoot` in order for `GlobalEventAware`ness to function; please call `.ui_root()` on your ui root")
                    }
                }))
                .on_despawn(move |world, _| {
                    world.commands().queue(clone!((observer_holder) move |world: &mut World| {
                        world.despawn(*observer_holder);
                    }))
                })
        })
    }
}

/// Container for global event data, including the original target entity and the event itself.
pub struct GlobalEventData<E> {
    /// The entity that originally received/triggered the event.
    pub original_event_target: Entity,
    /// The actual event data.
    pub event: E,
}
