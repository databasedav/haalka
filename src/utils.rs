use std::time::Duration;

use async_io::Timer;
use bevy::{
    tasks::{AsyncComputeTaskPool, Task},
    ui::{node_bundles::NodeBundle, Val},
};
use futures_util::Future;

use crate::{El, Element};

pub async fn sleep(duration: Duration) {
    Timer::after(duration).await;
}

pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    AsyncComputeTaskPool::get().spawn(future)
}

// TODO: this is cringe, but i don't know else to do it for now ... see the `type_erase_attempt`
// branch for a failedish attempt
pub fn naive_type_erase<E: Element>(el: E) -> El<NodeBundle> {
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
        })
        .child(el)
}
