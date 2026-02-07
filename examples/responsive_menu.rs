//! - A simple game menu, with buttons that use a nine-patch system for design (i.e., composed of
//!   images for the corners and middle segments) and an image to the right of the buttons.
//! - For normal screen sizes, the menu is centered in the middle of the screen
//! - For 400px width and lower, the buttons fill the screen width and the image is above the
//!   buttons.

mod utils;
use utils::*;

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

#[derive(Resource, Clone, Copy, Deref, DerefMut)]
struct Width(f32);

#[derive(Resource, Clone, Deref, DerefMut)]
struct NineSliceTexture(Handle<Image>);

#[derive(Resource, Clone, Deref, DerefMut)]
struct NineSliceTextureAtlasLayout(Handle<TextureAtlasLayout>);

#[derive(Resource, Clone, Deref, DerefMut)]
struct MenuImage(Handle<Image>);

const BASE_SIZE: f32 = 600.;
const GAP: f32 = 10.;
const FONT_SIZE: f32 = 33.33;

fn nine_slice_el(frame_signal: impl Signal<Item = usize> + 'static) -> El<ImageNode> {
    El::<ImageNode>::new()
        .with_builder(|builder| {
            builder.on_spawn_with_system(
                |In(entity): In<Entity>,
                 mut commands: Commands,
                 texture: Res<NineSliceTexture>,
                 layout: Res<NineSliceTextureAtlasLayout>| {
                    commands.entity(entity).insert(
                        ImageNode::from_atlas_image(
                            texture.0.clone(),
                            TextureAtlas {
                                layout: layout.0.clone(),
                                index: 0,
                            },
                        )
                        .with_mode(NodeImageMode::Sliced(TextureSlicer {
                            border: BorderRect::all(24.0),
                            center_scale_mode: SliceScaleMode::Stretch,
                            sides_scale_mode: SliceScaleMode::Stretch,
                            max_corner_scale: 1.0,
                        })),
                    );
                },
            )
        })
        .on_signal_with_image_node(frame_signal.dedupe(), move |mut image, frame| {
            if let Some(atlas) = &mut image.texture_atlas {
                atlas.index = frame;
            }
        })
}

fn nine_slice_button() -> impl Element {
    let lazy_entity = LazyEntity::new();
    let pressed = signal::from_entity(lazy_entity.clone())
        .has_component::<Pressed>()
        .dedupe();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    nine_slice_el(signal::zip!(pressed, hovered).dedupe().map_in(|(pressed, hovered)| {
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
    .insert((Pickable::default(), Hoverable, Pressable))
    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
}

fn menu() -> impl Element {
    let width = signal::from_resource_changed::<Width>().map_in(deref_copied);
    let is_wide = width.clone().map_in(|width| width > 400.).dedupe();
    let image_el = || {
        El::<ImageNode>::new().with_builder(|builder| {
            builder.on_spawn_with_system(
                |In(entity): In<Entity>, mut commands: Commands, menu_image: Res<MenuImage>| {
                    commands.entity(entity).insert(ImageNode::new(menu_image.0.clone()));
                },
            )
        })
    };
    nine_slice_el(signal::once(3))
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
                                    signal::from_resource_changed::<Width>()
                                        .map_in(deref_copied)
                                        .map_in_ref(ToString::to_string)
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
    commands.insert_resource(NineSliceTexture(asset_server.load("panels.png")));
    commands.insert_resource(NineSliceTextureAtlasLayout(
        texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::new(32, 32), 4, 1, None, None)),
    ));
    commands.insert_resource(MenuImage(asset_server.load("icon.png")));
    commands.spawn(Camera2d);
}

fn on_resize(mut resize_events: MessageReader<WindowResized>, mut width: ResMut<Width>) {
    for event in resize_events.read() {
        **width = event.width;
    }
}
