#[doc(no_inline)]
pub use enclose::enclose as clone;

// Re-export jonmo's clone macro as well
#[doc(no_inline)]
pub use jonmo::prelude::clone as jonmo_clone;

use std::sync::{Arc, OnceLock};
use bevy_ecs::{prelude::*, system::{SystemId, SystemInput, IntoObserverSystem}};
use jonmo::builder::JonmoBuilder;

/// Marker [`Component`] for filtering `SystemId` `Entity`s managed by haalka.
#[derive(Component)]
pub struct HaalkaOneShotSystem;

/// Marker [`Component`] for filtering `Observer` `Entity`s managed by haalka.
#[derive(Component)]
pub struct HaalkaObserver;

/// Register a system in the world with a marker component for filtering.
pub fn register_system<I: SystemInput + 'static, O: 'static, Marker, S: IntoSystem<I, O, Marker> + 'static>(
    world: &mut World,
    system: S,
) -> SystemId<I, O> {
    let system = world.register_system(system);
    if let Ok(mut entity) = world.get_entity_mut(system.entity()) {
        entity.insert(HaalkaOneShotSystem);
    }
    system
}

/// Attach an observer to an entity with a marker component for filtering.
pub fn observe<E: Event, B: Bundle, Marker>(
    world: &mut World,
    entity: Entity,
    observer: impl IntoObserverSystem<E, B, Marker>,
) -> EntityWorldMut<'_> {
    world.spawn((Observer::new(observer).with_entity(entity), HaalkaObserver))
}

/// If [`Some`] [`System`] is returned by the `getter`, remove it from the [`World`] on element removal.
pub fn remove_system_on_remove<I: SystemInput + 'static, O: 'static>(
    getter: impl FnOnce() -> Option<SystemId<I, O>> + Send + Sync + 'static,
) -> impl FnOnce(JonmoBuilder) -> JonmoBuilder {
    |builder| {
        builder.on_despawn(move |world, _| {
            if let Some(system) = getter() {
                world.commands().queue(move |world: &mut World| {
                    let _ = world.unregister_system(system);
                })
            }
        })
    }
}

/// Remove the held system from the [`World`] on element removal.
pub fn remove_system_holder_on_remove<I: SystemInput + 'static, O: 'static>(
    system_holder: Arc<OnceLock<SystemId<I, O>>>,
) -> impl FnOnce(JonmoBuilder) -> JonmoBuilder {
    remove_system_on_remove(move || system_holder.get().copied())
}

cfg_if::cfg_if! {
    if #[cfg(feature = "debug")] {
        use bevy_input::prelude::*;
        use bevy_app::prelude::*;
        use bevy_ui::prelude::*;
        use bevy_ui_render::prelude::*;

        const OVERLAY_TOGGLE_KEY: KeyCode = KeyCode::F1;

        fn toggle_overlay(
            input: Res<ButtonInput<KeyCode>>,
            mut options: ResMut<UiDebugOptions>,
        ) {
            if input.just_pressed(OVERLAY_TOGGLE_KEY) {
                options.toggle();
            }
        }

        /// Plugin that enables toggling the UI debug overlay with F1.
        pub struct DebugUiPlugin;

        impl Plugin for DebugUiPlugin {
            fn build(&self, app: &mut App) {
                app.add_systems(Update, toggle_overlay.run_if(any_with_component::<IsDefaultUiCamera>));
            }
        }
    }
}
