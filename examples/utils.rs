#![allow(missing_docs)]
#![allow(dead_code)]

use rand::seq::IndexedRandom;
use std::sync::LazyLock;

use bevy::{
    app::prelude::*,
    color::{palettes::css::*, prelude::*},
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
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
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

        static FPS: LazyLock<Mutable<f64>> = LazyLock::new(default);

        fn update_fps(diagnostic: Res<DiagnosticsStore>) {
            if let Some(fps_diagnostic) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS)
                && let Some(cur) = fps_diagnostic.smoothed()
            {
                FPS.set(cur);
            }
        }

        static SHOW: LazyLock<Mutable<bool>> = LazyLock::new(default);

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
    if let Ok(entity) = cameras.single()
        && let Ok(mut entity) = commands.get_entity(entity)
    {
        entity.try_insert(IsDefaultUiCamera);
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
}

static COLORS: &[Color] = &[
    bevy::prelude::Color::Srgba(ALICE_BLUE),
    bevy::prelude::Color::Srgba(ANTIQUE_WHITE),
    bevy::prelude::Color::Srgba(AQUA),
    bevy::prelude::Color::Srgba(AQUAMARINE),
    bevy::prelude::Color::Srgba(AZURE),
    bevy::prelude::Color::Srgba(BEIGE),
    bevy::prelude::Color::Srgba(BISQUE),
    bevy::prelude::Color::Srgba(BLANCHED_ALMOND),
    bevy::prelude::Color::Srgba(BLUE_VIOLET),
    bevy::prelude::Color::Srgba(BROWN),
    bevy::prelude::Color::Srgba(BURLYWOOD),
    bevy::prelude::Color::Srgba(CADET_BLUE),
    bevy::prelude::Color::Srgba(CHARTREUSE),
    bevy::prelude::Color::Srgba(CHOCOLATE),
    bevy::prelude::Color::Srgba(CORAL),
    bevy::prelude::Color::Srgba(CORNFLOWER_BLUE),
    bevy::prelude::Color::Srgba(CORNSILK),
    bevy::prelude::Color::Srgba(CRIMSON),
    bevy::prelude::Color::Srgba(DARK_BLUE),
    bevy::prelude::Color::Srgba(DARK_CYAN),
    bevy::prelude::Color::Srgba(DARK_GOLDENROD),
    bevy::prelude::Color::Srgba(DARK_GRAY),
    bevy::prelude::Color::Srgba(DARK_GREEN),
    bevy::prelude::Color::Srgba(DARK_GREY),
    bevy::prelude::Color::Srgba(DARK_KHAKI),
    bevy::prelude::Color::Srgba(DARK_MAGENTA),
    bevy::prelude::Color::Srgba(DARK_OLIVEGREEN),
    bevy::prelude::Color::Srgba(DARK_ORANGE),
    bevy::prelude::Color::Srgba(DARK_ORCHID),
    bevy::prelude::Color::Srgba(DARK_RED),
    bevy::prelude::Color::Srgba(DARK_SALMON),
    bevy::prelude::Color::Srgba(DARK_SEA_GREEN),
    bevy::prelude::Color::Srgba(DARK_SLATE_BLUE),
    bevy::prelude::Color::Srgba(DARK_SLATE_GRAY),
    bevy::prelude::Color::Srgba(DARK_SLATE_GREY),
    bevy::prelude::Color::Srgba(DARK_TURQUOISE),
    bevy::prelude::Color::Srgba(DARK_VIOLET),
    bevy::prelude::Color::Srgba(DEEP_PINK),
    bevy::prelude::Color::Srgba(DEEP_SKY_BLUE),
    bevy::prelude::Color::Srgba(DIM_GRAY),
    bevy::prelude::Color::Srgba(DIM_GREY),
    bevy::prelude::Color::Srgba(DODGER_BLUE),
    bevy::prelude::Color::Srgba(FIRE_BRICK),
    bevy::prelude::Color::Srgba(FLORAL_WHITE),
    bevy::prelude::Color::Srgba(FOREST_GREEN),
    bevy::prelude::Color::Srgba(GAINSBORO),
    bevy::prelude::Color::Srgba(GHOST_WHITE),
    bevy::prelude::Color::Srgba(GOLD),
    bevy::prelude::Color::Srgba(GOLDENROD),
    bevy::prelude::Color::Srgba(GREEN_YELLOW),
    bevy::prelude::Color::Srgba(GREY),
    bevy::prelude::Color::Srgba(HONEYDEW),
    bevy::prelude::Color::Srgba(HOT_PINK),
    bevy::prelude::Color::Srgba(INDIAN_RED),
    bevy::prelude::Color::Srgba(INDIGO),
    bevy::prelude::Color::Srgba(IVORY),
    bevy::prelude::Color::Srgba(KHAKI),
    bevy::prelude::Color::Srgba(LAVENDER),
    bevy::prelude::Color::Srgba(LAVENDER_BLUSH),
    bevy::prelude::Color::Srgba(LAWN_GREEN),
    bevy::prelude::Color::Srgba(LEMON_CHIFFON),
    bevy::prelude::Color::Srgba(LIGHT_BLUE),
    bevy::prelude::Color::Srgba(LIGHT_CORAL),
    bevy::prelude::Color::Srgba(LIGHT_CYAN),
    bevy::prelude::Color::Srgba(LIGHT_GOLDENROD_YELLOW),
    bevy::prelude::Color::Srgba(LIGHT_GRAY),
    bevy::prelude::Color::Srgba(LIGHT_GREEN),
    bevy::prelude::Color::Srgba(LIGHT_GREY),
    bevy::prelude::Color::Srgba(LIGHT_PINK),
    bevy::prelude::Color::Srgba(LIGHT_SALMON),
    bevy::prelude::Color::Srgba(LIGHT_SEA_GREEN),
    bevy::prelude::Color::Srgba(LIGHT_SKY_BLUE),
    bevy::prelude::Color::Srgba(LIGHT_SLATE_GRAY),
    bevy::prelude::Color::Srgba(LIGHT_SLATE_GREY),
    bevy::prelude::Color::Srgba(LIGHT_STEEL_BLUE),
    bevy::prelude::Color::Srgba(LIGHT_YELLOW),
    bevy::prelude::Color::Srgba(LIMEGREEN),
    bevy::prelude::Color::Srgba(LINEN),
    bevy::prelude::Color::Srgba(MAGENTA),
    bevy::prelude::Color::Srgba(MEDIUM_AQUAMARINE),
    bevy::prelude::Color::Srgba(MEDIUM_BLUE),
    bevy::prelude::Color::Srgba(MEDIUM_ORCHID),
    bevy::prelude::Color::Srgba(MEDIUM_PURPLE),
    bevy::prelude::Color::Srgba(MEDIUM_SEA_GREEN),
    bevy::prelude::Color::Srgba(MEDIUM_SLATE_BLUE),
    bevy::prelude::Color::Srgba(MEDIUM_SPRING_GREEN),
    bevy::prelude::Color::Srgba(MEDIUM_TURQUOISE),
    bevy::prelude::Color::Srgba(MEDIUM_VIOLET_RED),
    bevy::prelude::Color::Srgba(MIDNIGHT_BLUE),
    bevy::prelude::Color::Srgba(MINT_CREAM),
    bevy::prelude::Color::Srgba(MISTY_ROSE),
    bevy::prelude::Color::Srgba(MOCCASIN),
    bevy::prelude::Color::Srgba(NAVAJO_WHITE),
    bevy::prelude::Color::Srgba(OLD_LACE),
    bevy::prelude::Color::Srgba(OLIVE_DRAB),
    bevy::prelude::Color::Srgba(ORANGE),
    bevy::prelude::Color::Srgba(ORANGE_RED),
    bevy::prelude::Color::Srgba(ORCHID),
    bevy::prelude::Color::Srgba(PALE_GOLDENROD),
    bevy::prelude::Color::Srgba(PALE_GREEN),
    bevy::prelude::Color::Srgba(PALE_TURQUOISE),
    bevy::prelude::Color::Srgba(PALE_VIOLETRED),
    bevy::prelude::Color::Srgba(PAPAYA_WHIP),
    bevy::prelude::Color::Srgba(PEACHPUFF),
    bevy::prelude::Color::Srgba(PERU),
    bevy::prelude::Color::Srgba(PINK),
    bevy::prelude::Color::Srgba(PLUM),
    bevy::prelude::Color::Srgba(POWDER_BLUE),
    bevy::prelude::Color::Srgba(REBECCA_PURPLE),
    bevy::prelude::Color::Srgba(ROSY_BROWN),
    bevy::prelude::Color::Srgba(ROYAL_BLUE),
    bevy::prelude::Color::Srgba(SADDLE_BROWN),
    bevy::prelude::Color::Srgba(SALMON),
    bevy::prelude::Color::Srgba(SANDY_BROWN),
    bevy::prelude::Color::Srgba(SEA_GREEN),
    bevy::prelude::Color::Srgba(SEASHELL),
    bevy::prelude::Color::Srgba(SIENNA),
    bevy::prelude::Color::Srgba(SKY_BLUE),
    bevy::prelude::Color::Srgba(SLATE_BLUE),
    bevy::prelude::Color::Srgba(SLATE_GRAY),
    bevy::prelude::Color::Srgba(SLATE_GREY),
    bevy::prelude::Color::Srgba(SNOW),
    bevy::prelude::Color::Srgba(SPRING_GREEN),
    bevy::prelude::Color::Srgba(STEEL_BLUE),
    bevy::prelude::Color::Srgba(TAN),
    bevy::prelude::Color::Srgba(THISTLE),
    bevy::prelude::Color::Srgba(TOMATO),
    bevy::prelude::Color::Srgba(TURQUOISE),
    bevy::prelude::Color::Srgba(VIOLET),
    bevy::prelude::Color::Srgba(WHEAT),
    bevy::prelude::Color::Srgba(WHITE_SMOKE),
    bevy::prelude::Color::Srgba(YELLOW_GREEN),
];

pub(crate) fn random_color() -> Color {
    let mut rng = rand::rng();
    COLORS.choose(&mut rng).copied().unwrap()
}
