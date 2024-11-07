use std::time::Duration;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use bevy::tasks::Task;
    }
}
use bevy::{
    app::{App, Plugin},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::{resource_exists, TextBundle},
    tasks::IoTaskPool,
    utils::default,
    window::{MonitorSelection, Window, WindowPlugin, WindowPosition},
};
use cfg_if::cfg_if;
#[doc(no_inline)]
pub use enclose::enclose as clone;
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use gloo_timers::future::TimeoutFuture;
        use super::node_builder::WasmTaskAdapter;
    }
}
use haalka_futures_signals_ext::{futures_util::future::abortable, SignalExtExt};
use std::{future::Future, ops::Not};

use crate::raw::RawElWrapper;

/// Block for the `duration`.
pub async fn sleep(duration: Duration) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            send_wrapper::SendWrapper::new(TimeoutFuture::new(duration.as_millis().try_into().unwrap())).await;
        } else {
            async_io::Timer::after(duration).await;
        }
    }
}

// TODO: 0.15 `Task` api is unified, can remove branching
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
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
