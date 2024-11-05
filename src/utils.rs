use std::time::Duration;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use async_io::Timer;
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
#[doc(no_inline)]
pub use enclose::enclose as clone;
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use haalka_futures_signals_ext::{future::AbortHandle, futures_util::future::abortable, SignalExtExt};
use std::{future::Future, ops::Not};

use crate::raw::RawElWrapper;

/// Block for the `duration`.
pub async fn sleep(duration: Duration) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            send_wrapper::SendWrapper::new(TimeoutFuture::new(duration.as_millis().try_into().unwrap())).await;
        } else {
            Timer::after(duration).await;
        }
    }
}

// TODO: 0.15 `Task` api is unified, can remove branching
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Spawn a non-blocking future onto the [`IoTaskPool`].
        pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> AbortHandle {
            let (future, handle) = abortable(future);
            IoTaskPool::get().spawn(future);
            handle
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
    if #[cfg(debug_assertions)] {
        pub fn example_window() -> WindowPlugin {
            WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }
        }

        use bevy::{ui::prelude::*, text::prelude::*, app::prelude::*, ecs::prelude::*, input::prelude::*};
        use haalka_futures_signals_ext::SignalExtBool;
        use once_cell::sync::Lazy;
        use super::{element::Element, el::El, row::Row, raw::Spawnable};

        /// [`haalka`](crate) port of bevy::dev_tools::fps_overlay::FpsOverlayPlugin.
        #[derive(Default)]
        pub struct FpsOverlayPlugin;

        const FPS_FONT_SIZE: f32 = 20.;
        const FPS_TOGGLE_KEY: KeyCode = KeyCode::F1;
        const FPS_PADDING: f32 = 5.;
        pub const FPS_OVERLAY_ZINDEX: i32 = i32::MAX - 32;

        impl Plugin for FpsOverlayPlugin {
            fn build(&self, app: &mut App) {
                if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
                    app.add_plugins(FrameTimeDiagnosticsPlugin);
                }

                fn text(text: impl ToString) -> Text {
                    Text::from_section(
                        text.to_string(),
                        TextStyle {
                            font_size: FPS_FONT_SIZE,
                            ..default()
                        },
                    )
                }

                fn fps_element(fps: impl Signal<Item = f64> + Send + 'static) -> impl Element {
                    Row::<NodeBundle>::new()
                    // TODO: good place to use the text section signal abstraction, since doing a .text(...).text(...) does not work as expected
                    .item(El::<TextBundle>::new().text(text("fps: ")))
                    .item(El::<TextBundle>::new().text_signal(fps.map(|fps| format!("{fps:.2}")).map(text)))
                }

                static FPS: Lazy<Mutable<f64>> = Lazy::new(default);

                fn update_fps(diagnostic: Res<DiagnosticsStore>) {
                    if let Some(fps_diagnostic) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
                        if let Some(cur) = fps_diagnostic.smoothed() {
                            FPS.set(cur);
                        }
                    }
                }

                static SHOW: Lazy<Mutable<bool>> = Lazy::new(default);

                fn fps_ui_root() -> impl Element {
                    El::<NodeBundle>::new()
                        .with_style(|mut style| {
                            style.position_type = PositionType::Absolute;
                            style.padding.top = Val::Px(FPS_PADDING);
                            style.padding.left = Val::Px(FPS_PADDING);
                        })
                        .update_raw_el(|raw_el| raw_el.insert(ZIndex::Global(FPS_OVERLAY_ZINDEX)))
                        .child_signal(
                            SHOW.signal().map_true(move ||
                                fps_element(FPS.signal())
                            )
                        )
                }

                fn toggle_overlay(input: Res<ButtonInput<KeyCode>>, mut commands: Commands, fps_overlay_enabled_option: Option<Res<FpsOverlayEnabled>>) {
                    if input.just_pressed(FPS_TOGGLE_KEY) {
                        let exists = fps_overlay_enabled_option.is_some();
                        if exists {
                            commands.remove_resource::<FpsOverlayEnabled>();
                        } else {
                            commands.insert_resource(FpsOverlayEnabled);
                        }
                        SHOW.set_neq(!exists);
                    }
                }

                #[derive(Resource)]
                struct FpsOverlayEnabled;

                app
                    .add_systems(Startup, |world: &mut World| { fps_ui_root().spawn(world); })
                    .add_systems(Update, (toggle_overlay, update_fps.run_if(resource_exists::<FpsOverlayEnabled>)))
                    ;
            }
        }
    }
}
