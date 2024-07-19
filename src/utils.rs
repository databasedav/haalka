use std::time::Duration;

use async_io::Timer;
use bevy::tasks::{IoTaskPool, Task};
pub use enclose::enclose as clone;
use futures_signals::signal::{Mutable, Signal};
use haalka_futures_signals_ext::SignalExtExt;
use std::{future::Future, ops::Not};

/// Block for the `duration`.
pub async fn sleep(duration: Duration) {
    Timer::after(duration).await;
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
