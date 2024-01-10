use std::time::Duration;

use async_io::Timer;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_util::Future;

pub async fn sleep(duration: Duration) {
    Timer::after(duration).await;
}

pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}
