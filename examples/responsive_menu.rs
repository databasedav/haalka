//! - A simple game menu, with buttons that use a nine-patch system for design (i.e., composed of
//!   images for the corners and middle segments) and an image to the right of the buttons.
//! - For normal screen sizes, the menu is centered in the middle of the screen
//! - For 400px width and lower, the buttons fill the screen width and the image is above the
//!   buttons.

mod utils;
use utils::*;

use std::sync::OnceLock;

use bevy::{
    prelude::*,
    ui::{Pressed, widget::NodeImageMode},
    window::WindowResized,
};
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
        .insert_resource(Width(0.))
        .run();
}

#[derive(Resource, Clone, Copy, PartialEq, Deref, DerefMut)]
struct Width(f32);

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

fn nine_slice_el(frame_signal: impl Signal<Item = usize> + Send + Sync + 'static) -> El<ImageNode> {
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
    let lazy_entity = LazyEntity::new();
    let pressed_hovered_signal = SignalBuilder::from_lazy_entity(lazy_entity.clone())
        .map(|In(entity), presseds: Query<&Pressed>| presseds.contains(entity))
        .dedupe()
        .combine(
            SignalBuilder::from_lazy_entity(lazy_entity.clone())
                .has_component::<Hovered>()
                .dedupe(),
        )
        .dedupe();

    nine_slice_el(pressed_hovered_signal.clone().map_in(|(pressed, hovered)| {
        if pressed {
            2
        } else if hovered {
            1
        } else {
            0
        }
    }))
    .lazy_entity(lazy_entity)
    .with_node(|mut node| {
        node.width = Val::Px(100.);
        node.height = Val::Px(50.);
    })
    .insert(Pickable::default())
    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
}

fn menu() -> impl Element {
    let width = SignalBuilder::from_resource::<Width>().dedupe().map_in(deref_copied);
    let is_wide = width.clone().map_in(|width| width > 400.).dedupe();
    let image_el = || El::<ImageNode>::new().image_node(ImageNode::new(image().clone()));
    nine_slice_el(SignalBuilder::always(3))
        .with_node(|mut node| {
            node.height = Val::Px(BASE_SIZE);
            node.padding = UiRect::all(Val::Px(GAP));
        })
        .on_signal_with_node(
            width.map_in(|width| BASE_SIZE.min(width)).dedupe().map_in(Val::Px),
            |mut node, width| node.width = width,
        )
        .child(
            Stripe::<Node>::new()
                .direction_signal(
                    is_wide
                        .clone()
                        .map_bool_in(|| stripe::Direction::Row, || stripe::Direction::Column),
                )
                .with_node(|mut node| {
                    node.width = Val::Percent(100.);
                    node.height = Val::Percent(100.);
                    node.column_gap = Val::Px(GAP);
                    node.row_gap = Val::Px(GAP);
                })
                .item_signal(is_wide.clone().not().map_true_in(image_el))
                .item(
                    Stripe::<Node>::new()
                        .direction_signal(
                            is_wide
                                .clone()
                                .map_bool_in(|| stripe::Direction::Column, || stripe::Direction::Row),
                        )
                        .multiline_row_signal(is_wide.clone().not())
                        .align_content(Align::center())
                        .on_signal_with_node(is_wide.clone().dedupe(), |mut node, wide| {
                            if wide {
                                node.width = Val::Percent(50.);
                                node.height = Val::Percent(100.);
                                node.row_gap = Val::Px(GAP);
                                node.column_gap = Val::Px(0.);
                            } else {
                                node.width = Val::Percent(100.);
                                node.height = Val::Percent(50.);
                                node.column_gap = Val::Px(GAP);
                                node.row_gap = Val::Px(0.);
                            }
                        })
                        .items((0..8).map(|_| nine_slice_button())),
                )
                .item_signal(is_wide.clone().map_true_in(image_el)),
        )
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .insert(Pickable::default())
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
                                .text_signal(
                                    SignalBuilder::from_resource::<Width>()
                                        .map_in(deref_copied)
                                        .map_in_ref(|width: &f32| width.to_string())
                                        .map_in(Text)
                                        .map_in(Some),
                                ),
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

fn on_resize(mut resize_events: MessageReader<WindowResized>, mut width: ResMut<Width>) {
    for event in resize_events.read() {
        **width = event.width;
    }
}
