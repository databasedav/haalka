use std::time::Duration;

use async_io::Timer;
use bevy::{
    ecs::{system::SystemId, world::World},
    tasks::{IoTaskPool, Task},
};
pub use enclose::enclose as clone;
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
use haalka_futures_signals_ext::SignalExtExt;
use std::{future::Future, ops::Not};

use crate::RawHaalkaEl;

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

/// [`Signal`] outputing if two [`Signal`]s are equal.
pub fn signal_eq<T: PartialEq + Send>(
    signal_1: impl Signal<Item = T> + Send + 'static,
    signal_2: impl Signal<Item = T> + Send + 'static,
) -> impl Signal<Item = bool> + Send + 'static {
    map_ref!(signal_1, signal_2 => *signal_1 == *signal_2).dedupe()
}

/// If [`Some`] [`System`](bevy::ecs::system::System) is returned by the `getter`, remove it from
/// the [`World`] on entity removal.
pub fn remove_system_on_remove<I: 'static, O: 'static>(
    getter: impl FnOnce() -> Option<SystemId<I, O>> + Send + Sync + 'static,
) -> impl FnOnce(RawHaalkaEl) -> RawHaalkaEl {
    |raw_el| {
        raw_el.on_remove(move |world, _| {
            if let Some(system) = getter() {
                world.commands().add(move |world: &mut World| {
                    let _ = world.remove_system(system);
                })
            }
        })
    }
}

/// Remove the held system from the [`World`] on entity removal.
pub fn remove_system_holder_on_remove<I: 'static, O: 'static>(
    system_holder: Mutable<Option<SystemId<I, O>>>,
) -> impl FnOnce(RawHaalkaEl) -> RawHaalkaEl {
    remove_system_on_remove(move || system_holder.get())
}
