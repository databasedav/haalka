use std::{pin::Pin, time::Duration};

use bevy_tasks::{prelude::*, *};
#[doc(no_inline)]
pub use enclose::enclose as clone;
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
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

/// Spawn a non-blocking future onto the [`IoTaskPool`].
pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    IoTaskPool::get().spawn(future)
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

// TODO: get these from futures-signals https://github.com/Pauan/rust-signals/pull/85
pub type SyncBoxSignal<'a, T> = Pin<Box<dyn Signal<Item = T> + Send + Sync + 'a>>;

pub fn boxed_sync<'a, S, T>(signal: S) -> Pin<Box<dyn Signal<Item = T> + Send + Sync + 'a>>
where
    S: Sized + Send + Sync + Signal<Item = T> + 'a,
{
    Box::pin(signal)
}

cfg_if::cfg_if! {
    if #[cfg(feature = "debug")] {
        use bevy_ecs::prelude::*;
        use bevy_input::prelude::*;
        use bevy_app::prelude::*;
        use bevy_ui::prelude::*;

        const OVERLAY_TOGGLE_KEY: KeyCode = KeyCode::F1;

        fn toggle_overlay(
            input: Res<ButtonInput<KeyCode>>,
            mut options: ResMut<UiDebugOptions>,
        ) {
            if input.just_pressed(OVERLAY_TOGGLE_KEY) {
                options.toggle();
            }
        }

        pub struct DebugUiPlugin;

        impl Plugin for DebugUiPlugin {
            fn build(&self, app: &mut App) {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "ui")] {
                        app.add_systems(Update, toggle_overlay.run_if(any_with_component::<IsDefaultUiCamera>));
                    }
                }
            }
        }
    }
}
