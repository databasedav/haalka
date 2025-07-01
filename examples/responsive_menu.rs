//! - A simple game menu, with buttons that use a nine-patch system for design (i.e., composed of
//!   images for the corners and middle segments) and an image to the right of the buttons.
//! - For normal screen sizes, the menu is centered in the middle of the screen
//! - For 400px width and lower, the buttons fill the screen width and the image is above the
//!   buttons.

mod utils;
use bevy_ui::widget::NodeImageMode;
use utils::*;

use std::sync::OnceLock;

use bevy::{prelude::*, window::WindowResized};
use futures_signals::signal::Mutable;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(
            Startup,
            (setup, |world: &mut World| {
                ui_root().spawn(world);
            })
                .chain(),
        )
        .add_systems(Update, on_resize)
        .run();
}

const BASE_SIZE: f32 = 600.;
const GAP: f32 = 10.;
const FONT_SIZE: f32 = 33.33;

static NINE_SLICE_TEXTURE: OnceLock<Handle<Image>> = OnceLock::new();

fn nine_slice_texture() -> &'static Handle<Image> {
    NINE_SLICE_TEXTURE
        .get()
        .expect("expected NINE_SLICE_TEXTURE_ATLAS to be initialized")
}

static NINE_SLICE_TEXTURE_ATLAS_LAYOUT: OnceLock<Handle<TextureAtlasLayout>> = OnceLock::new();

fn nine_slice_texture_atlas_layout() -> &'static Handle<TextureAtlasLayout> {
    NINE_SLICE_TEXTURE_ATLAS_LAYOUT
        .get()
        .expect("expected NINE_SLICE_TEXTURE_ATLAS_LAYOUT to be initialized")
}

static IMAGE: OnceLock<Handle<Image>> = OnceLock::new();

fn image() -> &'static Handle<Image> {
    IMAGE.get().expect("expected IMAGE to be initialized")
}

fn nine_slice_el(frame_signal: impl Signal<Item = usize> + Send + 'static) -> El<ImageNode> {
    El::<ImageNode>::new()
        .image_node(
            ImageNode::from_atlas_image(
                nine_slice_texture().clone(),
                TextureAtlas {
                    layout: nine_slice_texture_atlas_layout().clone(),
                    index: 0,
                },
            )
            .with_mode(NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect::all(24.0),
                center_scale_mode: SliceScaleMode::Stretch,
                sides_scale_mode: SliceScaleMode::Stretch,
                max_corner_scale: 1.0,
            })),
        )
        .on_signal_with_image_node(frame_signal, move |mut image, frame| {
            if let Some(atlas) = &mut image.texture_atlas {
                atlas.index = frame;
            }
        })
}

fn nine_slice_button() -> impl Element {
    let hovered = Mutable::new(false);
    let pressed = Mutable::new(false);
    nine_slice_el(map_ref! {
        let hovered = hovered.signal(),
        let pressed = pressed.signal() => {
            if *pressed {
                2
            } else if *hovered {
                1
            } else {
                0
            }
        }
    })
    .width(Val::Px(100.))
    .height(Val::Px(50.))
    .hovered_sync(hovered)
    .pressed_sync(pressed)
    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
}

static WIDTH: LazyLock<Mutable<f32>> = LazyLock::new(default);

fn horizontal() -> impl Element {
    Row::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_node(|mut node| node.column_gap = Val::Px(GAP))
        .item(
            Column::<Node>::new()
                .width(Val::Percent(50.))
                .height(Val::Percent(100.))
                .with_node(|mut node| node.row_gap = Val::Px(GAP))
                .align_content(Align::center())
                .items((0..8).map(|_| nine_slice_button())),
        )
        .item(El::<ImageNode>::new().image_node(ImageNode::new(image().clone())))
}

fn vertical() -> impl Element {
    Column::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_node(|mut node| node.row_gap = Val::Px(GAP))
        .item(El::<ImageNode>::new().image_node(ImageNode::new(image().clone())))
        .item(
            Row::<Node>::new()
                .multiline()
                .align_content(Align::center())
                .width(Val::Percent(100.))
                .height(Val::Percent(50.))
                .with_node(|mut node| node.column_gap = Val::Px(GAP))
                .items((0..8).map(|_| nine_slice_button())),
        )
}

fn menu() -> impl Element {
    nine_slice_el(always(3))
        .height(Val::Px(BASE_SIZE))
        .with_node(|mut node| {
            node.padding = UiRect::all(Val::Px(GAP));
        })
        .width_signal(WIDTH.signal().map(|width| BASE_SIZE.min(width)).dedupe().map(Val::Px))
        .child_signal(
            WIDTH
                .signal()
                .map(|width| width > 400.)
                .dedupe()
                .map_bool(|| horizontal().type_erase(), || vertical().type_erase()),
        )
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .cursor(CursorIcon::default())
        .child(
            Column::<Node>::new()
                .with_node(|mut node| node.row_gap = Val::Px(GAP))
                .item(
                    Row::<Node>::new()
                        .with_node(|mut node| node.padding.left = Val::Px(GAP))
                        .item(
                            El::<Text>::new()
                                .text_font(TextFont::from_font_size(FONT_SIZE))
                                .text(Text::new("width: ")),
                        )
                        .item(
                            El::<Text>::new()
                                .text_font(TextFont::from_font_size(FONT_SIZE))
                                .text_signal(WIDTH.signal_ref(ToString::to_string).map(Text)),
                        ),
                )
                .item(menu()),
        )
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    NINE_SLICE_TEXTURE
        .set(asset_server.load("panels.png"))
        .expect("failed to initialize NINE_SLICE_TEXTURE");
    NINE_SLICE_TEXTURE_ATLAS_LAYOUT
        .set(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::new(32, 32), 4, 1, None, None)))
        .expect("failed to initialize NINE_SLICE_TEXTURE_ATLAS_LAYOUT");
    IMAGE
        .set(asset_server.load("icon.png"))
        .expect("failed to initialize IMAGE");
    commands.spawn(Camera2d);
}

fn on_resize(mut resize_events: EventReader<WindowResized>) {
    for event in resize_events.read() {
        WIDTH.set(event.width)
    }
}
