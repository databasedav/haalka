// A simple game menu, with buttons that use a nine-patch system for design (i.e., composed of
//     images for the corners and middle segments) and an image to the right of the buttons.
// For normal screen sizes, the menu is centered in the middle of the screen
// For 400px width and lower, the buttons fill the screen width and the image is above the buttons.

use std::sync::OnceLock;

use bevy::{prelude::*, window::WindowResized};
use bevy_nine_slice_ui::{prelude::*, NineSliceUiMaterialBundle};
use futures_signals::signal::Mutable;
use haalka::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        position: WindowPosition::Centered(MonitorSelection::Primary),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            HaalkaPlugin,
            NineSliceUiPlugin::default(),
        ))
        .add_systems(Startup, (setup, ui_root).chain())
        .add_systems(Update, on_resize)
        .insert_resource(Width(Mutable::new(0.)))
        .run();
}

const BASE_SIZE: f32 = 600.;
const GAP: f32 = 10.;

static NINE_SLICE_TEXTURE_ATLAS: OnceLock<Handle<Image>> = OnceLock::new();

pub fn nine_slice_texture_atlas() -> &'static Handle<Image> {
    NINE_SLICE_TEXTURE_ATLAS
        .get()
        .expect("expected NINE_SLICE_TEXTURE_ATLAS to be initialized")
}

static IMAGE: OnceLock<Handle<Image>> = OnceLock::new();

pub fn image() -> &'static Handle<Image> {
    IMAGE.get().expect("expected IMAGE to be initialized")
}

struct NineSliceEl<Bundle = NineSliceUiMaterialBundle>(El<Bundle>);

impl_haalka_methods! {
    NineSliceEl => {
        NineSliceUiMaterialBundle => {
            style: Style,
            nine_slice_texture: NineSliceUiTexture,
        },
    },
}

impl NineSliceEl {
    pub fn new(frame_signal: impl Signal<Item = usize> + Send + 'static) -> Self {
        Self(El::from(NineSliceUiMaterialBundle {
            nine_slice_texture: NineSliceUiTexture::from_slice(
                nine_slice_texture_atlas().clone(),
                Rect::new(0., 0., 32., 32.),
            ),
            ..default()
        }))
        .on_signal_with_nine_slice_texture(frame_signal, |nine_slice, frame| {
            if let Some(bounds) = &mut nine_slice.bounds {
                bounds.min.x = frame as f32 * 32.;
                bounds.max.x = 32. + frame as f32 * 32.;
            }
        })
    }
}

impl ElementWrapper for NineSliceEl {
    type EL = El<NineSliceUiMaterialBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.0
    }
}

impl PointerEventAware for NineSliceEl {}
impl Sizeable for NineSliceEl {}

fn nine_slice_button() -> impl Element {
    let hovered = Mutable::new(false);
    let pressed = Mutable::new(false);
    NineSliceEl::new(map_ref! {
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
}

#[derive(Resource)]
struct Width(Mutable<f32>);

fn horizontal() -> impl Element {
    Row::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_style(|style| style.column_gap = Val::Px(GAP))
        .item(
            Column::<NodeBundle>::new()
                .width(Val::Percent(50.))
                .height(Val::Percent(100.))
                .with_style(|style| style.row_gap = Val::Px(GAP))
                .align_content(Align::center())
                .items((0..8).into_iter().map(|_| nine_slice_button())),
        )
        .item(El::<ImageBundle>::new().image(UiImage::new(image().clone())))
}

fn vertical() -> impl Element {
    Column::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_style(|style| style.row_gap = Val::Px(GAP))
        .item(El::<ImageBundle>::new().image(UiImage::new(image().clone())))
        .item(
            Row::<NodeBundle>::new()
                .multiline()
                .align_content(Align::center())
                .width(Val::Percent(100.))
                .height(Val::Percent(50.))
                .with_style(|style| style.column_gap = Val::Px(GAP))
                .items((0..8).into_iter().map(|_| nine_slice_button())),
        )
}

fn menu(width: Mutable<f32>) -> impl Element {
    NineSliceEl::new(always(3))
        .height(Val::Px(BASE_SIZE))
        .with_style(|style| {
            style.padding = UiRect::all(Val::Px(GAP));
        })
        .width_signal(width.signal().map(|width| BASE_SIZE.min(width)).dedupe().map(Val::Px))
        .0
        .child_signal(
            width
                .signal()
                .map(|width| width > 400.)
                .dedupe()
                .map_bool(|| horizontal().type_erase(), || vertical().type_erase()),
        )
}

fn ui_root(world: &mut World) {
    let width = world.resource::<Width>().0.clone();
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Column::<NodeBundle>::new()
                .with_style(|style| style.row_gap = Val::Px(GAP))
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|style| style.padding.left = Val::Px(GAP))
                        .item(El::<TextBundle>::new().text(Text::from_section(
                            "width: ",
                            TextStyle {
                                font_size: 40.,
                                ..default()
                            },
                        )))
                        .item(El::<TextBundle>::new().text_signal(width.signal().map(|width| {
                            Text::from_section(
                                width.to_string(),
                                TextStyle {
                                    font_size: 40.,
                                    ..default()
                                },
                            )
                        }))),
                )
                .item(menu(width)),
        )
        .spawn(world);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    NINE_SLICE_TEXTURE_ATLAS
        .set(asset_server.load("panels.png"))
        .expect("failed to initialize NINE_SLICE_TEXTURE_ATLAS");
    IMAGE
        .set(asset_server.load("icon.png"))
        .expect("failed to initialize IMAGE");
    commands.spawn(Camera2dBundle::default());
}

fn on_resize(width: Res<Width>, mut resize_events: EventReader<WindowResized>) {
    for event in resize_events.read() {
        width.0.set(event.width)
    }
}
