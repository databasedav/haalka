use std::time::Duration;

use bevy_tasks::prelude::*;
#[doc(no_inline)]
pub use enclose::enclose as clone;
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use super::node_builder::WasmTaskAdapter;
    } else {
        use bevy_tasks::*;
    }
}
use haalka_futures_signals_ext::SignalExtExt;
use std::{future::Future, ops::Not};

/// Block for the `duration`.
pub async fn sleep(duration: Duration) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            send_wrapper::SendWrapper::new(gloo_timers::future::TimeoutFuture::new(duration.as_millis().try_into().unwrap())).await;
        } else {
            async_io::Timer::after(duration).await;
        }
    }
}

// TODO: 0.15 `Task` api is unified, can remove branching
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use haalka_futures_signals_ext::futures_util::future::abortable;
        /// Spawn a non-blocking future onto the [`IoTaskPool`].
        pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> WasmTaskAdapter {
            let (future, handle) = abortable(future);
            IoTaskPool::get().spawn(future);
            WasmTaskAdapter(handle)
        }
    } else {
        /// Spawn a non-blocking future onto the [`IoTaskPool`].
        pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
            IoTaskPool::get().spawn(future)
        }
    }
}

/// Sync the [`Mutable`] with the [`Signal`].
pub async fn sync<T>(signal: impl Signal<Item = T> + Send + 'static, mutable: Mutable<T>) {
    signal.for_each_sync(|value| mutable.set(value)).await;
}

/// Sync the [`Mutable`] with the [`Signal`] if the value has changed.
pub async fn sync_neq<T: PartialEq>(signal: impl Signal<Item = T> + Send + 'static, mutable: Mutable<T>) {
    signal.for_each_sync(|value| mutable.set_neq(value)).await;
}

/// Convenience utility for flipping the value of a [`Not`] mutable.
pub fn flip<T: Copy + Not<Output = T>>(mutable: &Mutable<T>) {
    let mut lock = mutable.lock_mut();
    *lock = lock.not();
}

/// [`Signal`] outputing if two [`Signal`]s are equal.
pub fn signal_eq<T: PartialEq + Send>(
    signal_1: impl Signal<Item = T> + Send + 'static,
    signal_2: impl Signal<Item = T> + Send + 'static,
) -> impl Signal<Item = bool> + Send + 'static {
    map_ref!(signal_1, signal_2 => *signal_1 == *signal_2).dedupe()
}

cfg_if::cfg_if! {
    if #[cfg(feature = "debug")] {
        use bevy_ecs::prelude::*;
        use bevy_input::prelude::*;
        use bevy_app::prelude::*;
        use bevy_ui::prelude::*;
        use bevy_dev_tools::ui_debug_overlay;

        const OVERLAY_TOGGLE_KEY: KeyCode = KeyCode::F1;

        fn toggle_overlay(
            input: Res<ButtonInput<KeyCode>>,
            mut options: ResMut<ui_debug_overlay::UiDebugOptions>,
        ) {
            if input.just_pressed(OVERLAY_TOGGLE_KEY) {
                options.toggle();
            }
        }

        pub struct DebugUiPlugin;

        cfg_if::cfg_if! {
            if #[cfg(feature = "text_input")] {
                use bevy_log::prelude::*;
                use bevy_cosmic_edit;

                #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
                pub struct CosmicMulticamHandlerSet;

                fn handle_cosmic_multicam(default_cameras: Query<Entity, With<IsDefaultUiCamera>>, mut commands: Commands) {
                    if let Ok(entity) = default_cameras.get_single() {
                        if let Some(mut entity) = commands.get_entity(entity) {
                            entity.try_insert(bevy_cosmic_edit::CosmicPrimaryCamera);
                            commands.remove_resource::<bevy_cosmic_edit::CursorPluginDisabled>();
                        }
                    } else {
                        warn!("DebugUiPlugin won't function without a camera with an IsDefaultUiCamera component");
                    }
                }
            }
        }

        impl Plugin for DebugUiPlugin {
            fn build(&self, app: &mut App) {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "ui")] {
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "text_input")] {
                                app
                                .insert_resource(bevy_cosmic_edit::CursorPluginDisabled)
                                .add_systems(PostStartup, handle_cosmic_multicam.in_set(CosmicMulticamHandlerSet))
                                .add_systems(Update, toggle_overlay.run_if(any_with_component::<IsDefaultUiCamera>.and_then(any_with_component::<bevy_cosmic_edit::CosmicPrimaryCamera>)));
                            } else {
                                app.add_systems(Update, toggle_overlay.run_if(any_with_component::<IsDefaultUiCamera>));
                            }
                        }
                    }
                }
                app.add_plugins(ui_debug_overlay::DebugUiPlugin);
            }
        }
    }
}
