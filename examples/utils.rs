#![allow(missing_docs)]
#![allow(dead_code)]

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
};
use bevy_window::WindowResolution;
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

        fn fps_element(fps: impl Signal<Item = f64> + Send + 'static) -> impl Element {
            Row::<Text>::new()
                // TODO: good place to use the text section signal abstraction, since doing a .text(...).text(...) does
                // not work as expected
                .item(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(FPS_FONT_SIZE))
                        .text(Text::new("fps: ")),
                )
                .item(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(FPS_FONT_SIZE))
                        .text_signal(fps.map(|fps| format!("{fps:.2}")).map(Text)),
                )
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
            El::<Node>::new()
                .with_node(|mut node| {
                    node.position_type = PositionType::Absolute;
                    node.padding.top = Val::Px(FPS_PADDING);
                    node.padding.left = Val::Px(FPS_PADDING);
                })
                .update_raw_el(|raw_el| raw_el.insert(GlobalZIndex(FPS_OVERLAY_ZINDEX)))
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

#[allow(clippy::type_complexity)]
fn mark_default_ui_camera(cameras: Query<Entity, Or<(With<Camera2d>, With<Camera3d>)>>, mut commands: Commands) {
    if let Ok(entity) = cameras.get_single() {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.try_insert(IsDefaultUiCamera);
        }
    }
}

pub(crate) const WINDOW_WIDTH: f32 = 1400.;
pub(crate) const WINDOW_HEIGHT: f32 = 900.;

pub(crate) fn examples_plugin(app: &mut App) {
    app.add_plugins((
        bevy::DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                position: WindowPosition::Centered(MonitorSelection::Primary),
                #[cfg(feature = "deployed_wasm_example")]
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
        (
            mark_default_ui_camera
                .in_set(MarkDefaultUiCameraSet)
                .run_if(not(any_with_component::<IsDefaultUiCamera>)),
            |world: &mut World| {
                let mut el = Column::<Node>::new()
                    .align(Align::new().bottom().left())
                    .with_node(|mut node| node.row_gap = Val::Px(10.));
                cfg_if::cfg_if! {
                    if #[cfg(feature = "debug")] {
                        el = el.item(El::<Text>::new().text(Text::new("press f1 to toggle debug overlay")));
                    }
                }
                el = el.item(El::<Text>::new().text(Text::new("press f2 to toggle fps counter")));
                El::<Node>::new()
                    .with_node(|mut node| {
                        node.padding.bottom = Val::Px(FPS_PADDING);
                        node.padding.left = Val::Px(FPS_PADDING);
                    })
                    .height(Val::Percent(100.))
                    .width(Val::Percent(100.))
                    .child(el)
                    .spawn(world);
            },
        ),
    );
    cfg_if::cfg_if! {
        if #[cfg(all(feature = "text_input", feature = "debug"))] {
            use haalka::utils::CosmicMulticamHandlerSet;
            app.configure_sets(PostStartup, (MarkDefaultUiCameraSet, CosmicMulticamHandlerSet).chain());
        }
    }
}

// TODO: this was needed otherwise clippy complains; how else to organize example specific utils?
fn main() {}
