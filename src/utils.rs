use std::time::Duration;

use async_io::Timer;
use bevy::tasks::{AsyncComputeTaskPool, Task};
pub use enclose::enclose as clone;
use std::future::Future;

/// Block for the `duration`.
pub async fn sleep(duration: Duration) {
    Timer::after(duration).await;
}

/// Spawn a future onto the [`AsyncComputeTaskPool`].
pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}
