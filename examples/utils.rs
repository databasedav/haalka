#[allow(missing_docs)]
#[allow(dead_code)]
use std::time::Duration;

use bevy::{
    app::prelude::*,
    core_pipeline::prelude::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::prelude::*,
    input::prelude::*,
    text::prelude::*,
    ui::prelude::*,
    utils::prelude::*,
    window::prelude::*,
    winit::*,
};
use haalka::prelude::*;
use haalka_futures_signals_ext::SignalExtBool;
use once_cell::sync::Lazy;

// TODO: port https://github.com/mintlu8/bevy-rectray/blob/main/examples/accordion.rs
// requires tweening api
// TODO: port https://github.com/mintlu8/bevy-rectray/blob/main/examples/draggable.rs
// requires tweening api ?


/// [`haalka`](crate) port of bevy::dev_tools::fps_overlay::FpsOverlayPlugin.
#[derive(Default)]
pub struct FpsOverlayPlugin;

const FPS_FONT_SIZE: f32 = 20.;
const FPS_TOGGLE_KEY: KeyCode = KeyCode::F2;
const FPS_PADDING: f32 = 5.;
const FPS_OVERLAY_ZINDEX: i32 = i32::MAX - 32;

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
                // TODO: good place to use the text section signal abstraction, since doing a .text(...).text(...) does
                // not work as expected
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
                .child_signal(SHOW.signal().map_true(move || fps_element(FPS.signal())))
        }

        fn toggle_overlay(
            input: Res<ButtonInput<KeyCode>>,
            mut commands: Commands,
            fps_overlay_enabled_option: Option<Res<FpsOverlayEnabled>>,
        ) {
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

        app.add_systems(Startup, |world: &mut World| {
            fps_ui_root().spawn(world);
        })
        .add_systems(
            Update,
            (toggle_overlay, update_fps.run_if(resource_exists::<FpsOverlayEnabled>)),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct MarkDefaultUiCameraSet;

fn mark_default_ui_camera(cameras: Query<Entity, Or<(With<Camera2d>, With<Camera3d>)>>, mut commands: Commands) {
    if let Ok(entity) = cameras.get_single() {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.try_insert(IsDefaultUiCamera);
        }
    }
}

pub(crate) fn examples_plugin(app: &mut App) {
    app.add_plugins((
        bevy::DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                position: WindowPosition::Centered(MonitorSelection::Primary),
                // #[cfg(feature = "deployed_wasm_example")]
                canvas: Some("#bevy".to_string()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }),
        HaalkaPlugin,
        FpsOverlayPlugin,
        #[cfg(feature = "debug")]
        DebugUiPlugin,
    ))
    .add_systems(
        PostStartup,
        mark_default_ui_camera
            .in_set(MarkDefaultUiCameraSet)
            .run_if(not(any_with_component::<IsDefaultUiCamera>)),
    );
    cfg_if::cfg_if! {
        if #[cfg(all(feature = "text_input", feature = "debug"))] {
            use haalka::utils::CosmicMulticamHandlerSet;
            app.configure_sets(PostStartup, (MarkDefaultUiCameraSet, CosmicMulticamHandlerSet).chain());
        }
    }
    // cfg_if::cfg_if! {
    //     if #[cfg(target_arch = "wasm32")] {
    //         {
    //             const MAX_WASM_FPS: f32 = 240.;
    //             const LIMIT_FRAMERATE_TOGGLE_KEY: KeyCode = KeyCode::F3;

    //             static CAPPED_FRAMERATE_UPDATE_MODE: Lazy<UpdateMode> = Lazy::new(|| UpdateMode::Reactive {
    //                 wait: Duration::from_secs_f32(1. / MAX_WASM_FPS),
    //                 react_to_device_events: false,
    //                 react_to_user_events: false,
    //                 react_to_window_events: false,
    //             });

    //             fn toggle_framerate_cap(
    //                 input: Res<ButtonInput<KeyCode>>,
    //                 mut winit_settings: ResMut<WinitSettings>,
    //             ) {
    //                 if input.just_pressed(LIMIT_FRAMERATE_TOGGLE_KEY) {
    //                     if matches!(winit_settings.focused_mode, UpdateMode::Continuous) {
    //                         winit_settings.focused_mode = *CAPPED_FRAMERATE_UPDATE_MODE;
    //                     } else {
    //                         winit_settings.focused_mode = UpdateMode::Continuous;
    //                     }
    //                 }
    //             }

    //             app
    //             .add_systems(PostStartup, |mut winit_settings: ResMut<WinitSettings>| {
    //                 winit_settings.focused_mode = *CAPPED_FRAMERATE_UPDATE_MODE;
    //             })
    //             .add_systems(Update, toggle_framerate_cap);
    //         }
    //     }
    // };
}
