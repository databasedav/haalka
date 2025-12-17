//! - Fixed-size grid, some spaces with items and some empty.
//! - Each item slot has an image of the item and the item count overlayed on the image.
//! - Items can be moved with drag and drop.
//!   - Both image and item count move along with the cursor while dragging.
//!   - The image and item count are not visible in the original position while dragging.
//!   - You can leave the bounding box of the inventory while dragging.
//! - A tooltip with the item's name is shown when hovering over an item.

// TODO: fix cursor not updating when placing an item in an empty cell and then moving cursor
// outside

mod utils;
use utils::*;

use std::{collections::HashMap, sync::OnceLock};

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use haalka::{jonmo::graph::SignalHandle, prelude::*};
use rand::{
    Rng,
    distr::{Bernoulli, Distribution},
};

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .init_state::<AssetState>()
        .add_loading_state(
            LoadingState::new(AssetState::Loading)
                .continue_to_state(AssetState::Loaded)
                .load_collection::<RpgIconSheet>(),
        )
        .insert_resource(PointerPosition::default())
        .insert_resource(Dragging::default())
        // .add_systems(Startup, character_camera)
        // .add_systems(Startup, setup_3d)
        // .add_systems(Update, rotate_prism)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Camera2d, IsDefaultUiCamera));
        })
        .add_systems(
            OnEnter(AssetState::Loaded),
            (set_icon_texture_atlas, |world: &mut World| {
                let handles = InventoryHandles::new();
                let root = ui_root(handles.clone())
                    .with_builder(|builder| {
                        builder.on_spawn_with_system(
                            move |In(entity): In<_>,
                                  camera: Single<Entity, With<IsDefaultUiCamera>>,
                                  mut commands: Commands| {
                                // https://github.com/bevyengine/bevy/discussions/11223
                                if let Ok(mut commands) = commands.get_entity(entity) {
                                    commands.try_insert(UiTargetCamera(*camera));
                                }
                            },
                        )
                    })
                    .spawn(world);

                // Register crafting output updater after UI is spawned so LazyEntitys are set.
                let crafting_outputter = register_crafting_outputter(world, handles);
                if let Ok(mut entity) = world.get_entity_mut(root)
                    && let Some(mut signal_handles) = entity.get_mut::<SignalHandles>()
                {
                    signal_handles.add(crafting_outputter);
                }
            })
                .chain(),
        )
        .run();
}

const CELL_WIDTH: f32 = 70.;
const INVENTORY_BACKGROUND_COLOR: Color = Color::hsl(0., 0., 0.78);
const CELL_BACKGROUND_COLOR: Color = Color::hsl(0., 0., 0.55);
const CELL_HIGHLIGHT_COLOR: Color = Color::hsl(0., 0., 0.83);
const CELL_GAP: f32 = 5.;
const INVENTORY_SIZE: f32 = 700.;
const CELL_BORDER_WIDTH: f32 = 2.;
const CELL_DARK_BORDER_COLOR: Color = Color::hsl(0., 0., 0.19);
// const CELL_LIGHT_BORDER_COLOR: Color = Color::hsl(0., 0., 0.98);

static ITEM_NAMES: LazyLock<HashMap<usize, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (0, "copper dagger"),
        (1, "copper sword"),
        (2, "shortbow"),
        (3, "copper spear"),
        (4, "copper axe"),
        (5, "copper mace"),
        (6, "copper shovel"),
        (7, "copper pickaxe"),
        (8, "copper hammer"),
        (9, "copper scythe"),
        (10, "steel dagger"),
        (11, "steel sword"),
        (12, "longbow"),
        (13, "steel spear"),
        (14, "steel axe"),
        (15, "steel mace"),
        (16, "steel shovel"),
        (17, "steel pickaxe"),
        (18, "steel hammer"),
        (19, "steel scythe"),
        (20, "golden dagger"),
        (21, "golden sword"),
        (22, "golden longbow"),
        (23, "golden spear"),
        (24, "golden axe"),
        (25, "golden mace"),
        (26, "golden shovel"),
        (27, "golden pickaxe"),
        (28, "golden hammer"),
        (29, "golden scythe"),
        (30, "copper arrow"),
        (31, "steel arrow"),
        (32, "golden arrow"),
        (33, "poison arrow"),
        (34, "fire arrow"),
        (35, "ice arrow"),
        (36, "electric arrow"),
        (37, "charm arrow"),
        (38, "leather quiver"),
        (39, "elven quiver"),
        (40, "apprentice robes"),
        (41, "common shirt"),
        (42, "copper armor"),
        (43, "turtle buckler"),
        (44, "wooden shield"),
        (45, "plank shield"),
        (46, "shoes"),
        (47, "apprentice hat"),
        (48, "cloth cap"),
        (49, "copper helmet"),
        (50, "mage robes"),
        (51, "leather armor"),
        (52, "steel armor"),
        (53, "wooden buckler"),
        (54, "reinforced wooden shield"),
        (55, "steel shield"),
        (56, "leather boots"),
        (57, "mage hat"),
        (58, "leather helmet"),
        (59, "steel helmet"),
        (60, "archmage robes"),
        (61, "elven armor"),
        (62, "golden armor"),
        (63, "steel buckler"),
        (64, "steel round shield"),
        (65, "golden shield"),
        (66, "elven boots"),
        (67, "archmage hat"),
        (68, "elven helmet"),
        (69, "golden helmet"),
        (70, "wooden staff"),
        (71, "fire staff"),
        (72, "lightning staff"),
        (73, "ice staff"),
        (74, "fire ring"),
        (75, "lightning ring"),
        (76, "ice ring"),
        (77, "fire necklace"),
        (78, "lightning necklace"),
        (79, "ice necklace"),
        (80, "minor healing potion"),
        (81, "healing potion"),
        (82, "greater healing potion"),
        (83, "minor mana potion"),
        (84, "mana potion"),
        (85, "greater mana potion"),
        (86, "yellow potion"),
        (87, "green potion"),
        (88, "purple potion"),
        (89, "flying potion"),
        (90, "gold coins (small)"),
        (91, "gold coins (medium)"),
        (92, "gold coins (big)"),
        (93, "gold pouch"),
        (94, "gold chest"),
        (95, "ruby"),
        (96, "topaz"),
        (97, "emerald"),
        (98, "sapphire"),
        (99, "diamond"),
        (100, "map"),
        (101, "journal"),
        (102, "satchel"),
        (103, "backpack"),
        (104, "pouch"),
        (105, "chest (small)"),
        (106, "chest (big)"),
        (107, "bronze key"),
        (108, "silver key"),
        (109, "golden key"),
        (110, "wood log"),
        (111, "stone"),
        (112, "meat"),
        (113, "cheese"),
        (114, "apple"),
        (115, "poisoned apple"),
        (116, "milk glass"),
        (117, "egg (white)"),
        (118, "egg (brown)"),
        (119, "egg (golden)"),
        (120, "carrot"),
        (121, "berries"),
        (122, "sunflower"),
        (123, "flower (yellow)"),
        (124, "flower (blue)"),
        (125, "flower (red)"),
        (126, "fishing rod"),
        (127, "worm"),
        (128, "fish_1"),
        (129, "fish_2"),
    ])
});

// TODO: port to Lazy
static ICON_TEXTURE_ATLAS: OnceLock<RpgIconSheet> = OnceLock::new();

// using a global handle for this so we don't need to thread the texture atlas handle through the
// ui tree when we can guarantee it exists before any cells are inserted
fn icon_sheet() -> &'static RpgIconSheet {
    ICON_TEXTURE_ATLAS
        .get()
        .expect("expected ICON_TEXTURE_ATLAS to be initialized")
}

#[derive(AssetCollection, Resource, Clone, Debug)]
struct RpgIconSheet {
    #[asset(texture_atlas(tile_size_x = 48, tile_size_y = 48, columns = 10, rows = 27))]
    layout: Handle<TextureAtlasLayout>,
    #[asset(path = "rpg_icon_sheet.png")]
    image: Handle<Image>,
}

fn icon(
    index_signal: impl Signal<Item = usize> + Send + 'static,
    count_signal: impl Signal<Item = usize> + Send + 'static,
) -> Stack<Node> {
    Stack::new()
        .layer(
            El::<ImageNode>::new()
                .image_node(ImageNode {
                    image: icon_sheet().image.clone(),
                    texture_atlas: Some(TextureAtlas::from(icon_sheet().layout.clone())),
                    ..default()
                })
                .on_signal_with_image_node(index_signal, |mut image_node: Mut<ImageNode>, index| {
                    if let Some(ref mut texture_atlas) = image_node.texture_atlas {
                        texture_atlas.index = index;
                    }
                }),
        )
        .layer(
            El::<Text>::new()
                .with_node(|mut node| node.top = Val::Px(6.))
                .align(Align::new().bottom().right())
                .text_font(TextFont::from_font_size(33.33))
                .text_signal(count_signal.map_in_ref(ToString::to_string).map_in(Text).map_in(Some)),
        )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellData {
    index: usize,
    count: usize,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
struct CellContent(Option<CellData>);

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
struct PointerPosition(Vec2);

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
struct Dragging {
    item: Option<CellData>,
    source: Option<Entity>,
}

#[derive(Clone)]
struct InventoryHandles {
    craft_inputs: [LazyEntity; 4],
    craft_output: LazyEntity,
}

impl InventoryHandles {
    fn new() -> Self {
        Self {
            craft_inputs: [
                LazyEntity::new(),
                LazyEntity::new(),
                LazyEntity::new(),
                LazyEntity::new(),
            ],
            craft_output: LazyEntity::new(),
        }
    }
}

#[derive(Component, Clone)]
struct PressHandlingDisabled;

#[derive(Component, Clone)]
struct OutputPressDisabled;

#[derive(Component, Default)]
struct TooltipOrigin(Option<Vec2>);

fn is_dragging_signal() -> impl Signal<Item = bool> + Clone {
    SignalBuilder::from_resource::<Dragging>()
    .map_in(|Dragging { item, .. }| item.is_some())
        .dedupe()
}

fn pointer_position_signal() -> impl Signal<Item = Vec2> + Clone {
    SignalBuilder::from_resource::<PointerPosition>()
        .map_in(|PointerPosition(pos)| pos)
        .dedupe()
}

fn cell(initial: Option<CellData>, insertable: bool) -> impl Element {
    cell_with_handle(LazyEntity::new(), initial, insertable)
}

fn cell_with_handle(lazy_entity: LazyEntity, initial: Option<CellData>, insertable: bool) -> impl Element {
    let dragging = is_dragging_signal();
    let hovered = SignalBuilder::from_lazy_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let content = SignalBuilder::from_component_lazy::<CellContent>(lazy_entity.clone()).dedupe();
    let populated = content.clone().map_in(|CellContent(x)| x.is_some()).dedupe();

    let index_signal = content
        .clone()
        .map_in(|CellContent(x)| x.map(|d| d.index).unwrap_or(0))
        .dedupe();
    let count_signal = content
        .clone()
        .map_in(|CellContent(x)| x.map(|d| d.count).unwrap_or(0))
        .dedupe();

    let press_disabled = signal::or!(dragging.clone(), populated.clone().map_in(|p| !p).dedupe()).dedupe();

    let tooltip_enabled = signal::and!(
        hovered.clone(),
        populated.clone(),
        dragging.clone().map_in(|d| !d).dedupe(),
    )
    .dedupe();

    let icon_layer = icon(index_signal, count_signal)
        .with_builder(|builder| {
            builder.component_signal::<Visibility, _>(
                populated
                    .clone()
                    .map_in(|p| if p { Visibility::Inherited } else { Visibility::Hidden })
                    .map_in(Some),
            )
        })
        .type_erase();

    let tooltip = {
        let cell_entity = lazy_entity.clone();
        El::<Node>::new()
            .with_builder(|builder| builder.insert(TooltipOrigin::default()))
            .with_node(|mut node| {
                node.height = Val::Px(CELL_WIDTH);
                node.position_type = PositionType::Absolute;
                node.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
                node.padding = UiRect::horizontal(Val::Px(10.));
            })
            .visibility(Visibility::Hidden)
            .with_builder(|builder| {
                builder.on_signal_with_entity(pointer_position_signal(), move |mut entity, pointer| {
                    let Some(transform) = entity.get::<GlobalTransform>().cloned() else {
                        return;
                    };

                    // Initialize the tooltip's world-space origin once GlobalTransform is available.
                    let origin = if let Some(mut origin) = entity.get_mut::<TooltipOrigin>() {
                        if origin.0.is_none() {
                            origin.0 = Some(transform.compute_transform().translation.xy());
                        }
                        origin.0
                    } else {
                        None
                    };

                    if let Some(origin) = origin {
                        let left = pointer.x - (origin.x - CELL_WIDTH / 2.);
                        let top = pointer.y - (origin.y + CELL_WIDTH / 2.);

                        if let Some(mut node) = entity.get_mut::<Node>() {
                            node.left = Val::Px(left);
                            node.top = Val::Px(top);
                        }
                        // Avoid a one-frame flash at the default position.
                        entity.insert(Visibility::Visible);
                    }
                })
            })
            .global_z_index(GlobalZIndex(1))
            .background_color(BackgroundColor(CELL_BACKGROUND_COLOR))
            .border_color(BorderColor::all(CELL_DARK_BORDER_COLOR))
            .child(
                El::<Text>::new()
                    .align(Align::center())
                    .text_font(TextFont::from_font_size(41.67))
                    .text_layout(TextLayout::new_with_no_wrap())
                    .text_signal(
                        SignalBuilder::from_component_lazy::<CellContent>(cell_entity)
                            .map_in(|CellContent(x)| x.map(|d| d.index))
                            .dedupe()
                            .map_in(|index: Option<usize>| {
                                index
                                    .and_then(|i| ITEM_NAMES.get(&i).copied())
                                    .unwrap_or_default()
                                    .to_string()
                            })
                            .map_in(Text)
                            .map_in(Some),
                    ),
            )
            .type_erase()
    };

    El::<Node>::new()
        .with_builder(|builder| {
            let builder = builder
                .lazy_entity(lazy_entity.clone())
                .insert(CellContent(initial))
                .component_signal::<PressHandlingDisabled, _>(press_disabled.map_true_in(|| PressHandlingDisabled))
                .observe(
                    |pointer_down: On<Pointer<Press>>,
                     disabled: Query<&PressHandlingDisabled>,
                     mut dragging: ResMut<Dragging>,
                     mut pointer: ResMut<PointerPosition>,
                     mut contents: Query<&mut CellContent>| {
                        if disabled.contains(pointer_down.entity) {
                            return;
                        }

                        let mut to_drag: Option<CellData> = None;
                        if let Ok(mut content) = contents.get_mut(pointer_down.entity) {
                            match pointer_down.button {
                                PointerButton::Secondary => {
                                    if let Some(mut data) = content.0 {
                                        let to_take = (data.count / 2).max(1);
                                        data.count = data.count.saturating_sub(to_take);
                                        to_drag = Some(CellData {
                                            index: data.index,
                                            count: to_take,
                                        });
                                        content.0 = (data.count > 0).then_some(data);
                                    }
                                }
                                _ => {
                                    to_drag = content.0.take();
                                }
                            }
                        }

                        dragging.item = to_drag;
                        dragging.source = dragging.item.map(|_| pointer_down.entity);
                        pointer.0 = pointer_down.pointer_location.position;
                    },
                );

            if insertable {
                builder.insert(Pickable::default()).observe(
                    |click: On<Pointer<Click>>,
                     mut dragging: ResMut<Dragging>,
                     mut contents: Query<&mut CellContent>| {
                        let Some(mut dragged) = dragging.item else {
                            return;
                        };

                        // Ignore the click emitted by releasing the same press that started the
                        // drag on the source cell.
                        if dragging.source == Some(click.entity) {
                            return;
                        }

                        // Any other click is a deliberate drop action; stop treating the source
                        // cell specially.
                        dragging.source = None;

                        let Ok(mut content) = contents.get_mut(click.entity) else {
                            return;
                        };

                        match &mut content.0 {
                            None => {
                                if matches!(click.button, PointerButton::Secondary) {
                                    // Drop a single item into an empty slot.
                                    content.0 = Some(CellData {
                                        index: dragged.index,
                                        count: 1,
                                    });
                                    dragged.count = dragged.count.saturating_sub(1);
                                    dragging.item = (dragged.count > 0).then_some(dragged);
                                } else {
                                    // Drop entire stack into empty slot.
                                    content.0 = Some(dragged);
                                    dragging.item = None;
                                }
                            }
                            Some(existing) => {
                                if existing.index == dragged.index {
                                    if matches!(click.button, PointerButton::Secondary) {
                                        existing.count = existing.count.saturating_add(1);
                                        dragged.count = dragged.count.saturating_sub(1);
                                        dragging.item = (dragged.count > 0).then_some(dragged);
                                    } else {
                                        existing.count = existing.count.saturating_add(dragged.count);
                                        dragging.item = None;
                                    }
                                } else {
                                    // Swap different items.
                                    let tmp = *existing;
                                    *existing = dragged;
                                    dragging.item = Some(tmp);
                                }
                            }
                        }
                    },
                )
            } else {
                builder
            }
        })
        .cursor_signal(
            dragging
                .clone()
                .combine(populated.clone())
                .map_in(|(dragging, populated)| {
                    if dragging {
                        CursorIcon::System(SystemCursorIcon::Grabbing)
                    } else if populated {
                        CursorIcon::System(SystemCursorIcon::Grab)
                    } else {
                        CursorIcon::default()
                    }
                })
                .dedupe(),
        )
        .with_node(|mut node| {
            node.width = Val::Px(CELL_WIDTH);
            node.height = Val::Px(CELL_WIDTH);
            node.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
        })
        .background_color_signal(
            hovered
                .map_bool_in(|| CELL_HIGHLIGHT_COLOR, || CELL_BACKGROUND_COLOR)
                .map_in(BackgroundColor)
                .map_in(Some),
        )
        .border_color(BorderColor::all(CELL_DARK_BORDER_COLOR))
        .child(icon_layer)
        .child_signal(tooltip_enabled.map_true_in(move || tooltip.clone()))
}

fn random_cell_data(rng: &mut impl Rng) -> CellData {
    CellData {
        index: rng.random_range(0..ITEM_NAMES.len()),
        count: rng.random_range(1..=64),
    }
}

fn bern_cell_data_option(bern: f64) -> Option<CellData> {
    'block: {
        let distribution = Bernoulli::new(bern).unwrap();
        let mut rng = rand::rng();
        if distribution.sample(&mut rng) {
            break 'block Some(random_cell_data(&mut rng));
        }
        None
    }
}

fn bern_cell(bern: f64, insertable: bool) -> impl Element {
    cell(bern_cell_data_option(bern), insertable)
}

fn grid<I: IntoIterator<Item = Option<CellData>>>(cell_data_options: I) -> impl Element
where
    <I as IntoIterator>::IntoIter: std::marker::Send + 'static,
{
    Grid::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
            node.column_gap = Val::Px(CELL_GAP);
            node.row_gap = Val::Px(CELL_GAP);
        })
        .row_wrap_cell_width(CELL_WIDTH)
        .cells(
            cell_data_options
                .into_iter()
                .map(move |cell_data_option| cell(cell_data_option, true)),
        )
}

fn set_icon_texture_atlas(rpg_icon_sheet: Res<RpgIconSheet>) {
    ICON_TEXTURE_ATLAS
        .set(rpg_icon_sheet.clone())
        .expect("failed to initialize ICON_TEXTURE_ATLAS");
}

// fn character_camera(mut commands: Commands) {
//     // https://github.com/bevyengine/bevy/discussions/11223
//     commands.spawn((
//         Camera3d::default(),
//         Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
//         Camera {
//             order: 1,
//             clear_color: ClearColorConfig::None,
//             ..default()
//         },
//         RenderLayers::layer(1),
//     ));
// }

// fn setup_3d(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials:
// ResMut<Assets<StandardMaterial>>) {     // Add a light source
//     commands.spawn(PointLight {
//         intensity: 1500.0,
//         shadows_enabled: true,
//         ..default()
//     })
//     .insert(Transform::from_xyz(4.0, 8.0, 4.0));

//     // Spawn the rotating rectangular prism
//     commands.spawn((
//         Mesh3d(meshes.add(Cuboid::default())),
//         MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
//         Transform::from_scale(Vec3::new(1.0, 1.5, 0.5)),
//         RotatingPrism,
//         RenderLayers::layer(1),
//     ));
// }

// fn rotate_prism(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingPrism>>) {
//     for mut transform in query.iter_mut() {
//         transform.rotation *= Quat::from_rotation_y(1.0 * time.delta_secs());
//     }
// }

// #[derive(Component)]
// struct RotatingPrism;

fn dot() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(CELL_BORDER_WIDTH * 2.);
            node.height = Val::Px(CELL_BORDER_WIDTH * 2.);
        })
        .background_color(BackgroundColor(CELL_BACKGROUND_COLOR))
}

fn dot_row(n: usize) -> impl Element {
    Row::<Node>::new().items((0..n).map(|_| dot()))
}

fn arrow() -> impl Element {
    Column::<Node>::new()
        .align_content(Align::center())
        .items((0..=6).map(|i| dot_row(2 * i + 1)))
        .items((0..6).map(|_| dot_row(3)))
}

fn side_column() -> impl Element {
    Column::<Node>::new()
        .with_node(|mut node| node.row_gap = Val::Px(CELL_GAP))
        .items((0..4).map(|_| bern_cell(0.5, true)))
}

fn inventory_with_handles(handles: InventoryHandles) -> impl Element {
    El::<Node>::new()
        .align(Align::center())
        .with_node(|mut node| {
            node.height = Val::Px(INVENTORY_SIZE);
            node.width = Val::Px(INVENTORY_SIZE);
        })
        .child(
            Column::<Node>::new()
                .with_node(|mut node| {
                    node.height = Val::Percent(100.);
                    node.width = Val::Percent(100.);
                    node.row_gap = Val::Px(CELL_GAP * 4.);
                })
                .background_color(BackgroundColor(INVENTORY_BACKGROUND_COLOR))
                .align_content(Align::center())
                .item(
                    Row::<Node>::new()
                        .with_node(|mut node| {
                            node.width = Val::Percent(100.);
                            node.column_gap = Val::Px(CELL_GAP);
                        })
                        .item(
                            Row::<Node>::new()
                                .align_content(Align::center())
                                .with_node(|mut node| {
                                    node.width = Val::Percent(60.);
                                    node.column_gap = Val::Px(CELL_GAP);
                                    node.padding = UiRect::horizontal(Val::Px(CELL_GAP * 3.));
                                })
                                .item(side_column())
                                .item(
                                    El::<Node>::new()
                                        .with_node(|mut node| {
                                            node.height = Val::Px(CELL_WIDTH * 4. + CELL_GAP * 3.);
                                            node.width = Val::Percent(100.);
                                        })
                                        .background_color(BackgroundColor(Color::BLACK)),
                                )
                                .item(side_column())
                        )
                        .item(
                            El::<Node>::new()
                                .with_node(|mut node| {
                                    node.width = Val::Percent(40.);
                                    node.height = Val::Percent(100.);
                                })
                                .align_content(Align::center())
                                .child({
                                    Column::<Node>::new()
                                        .with_node(|mut node| {
                                            node.row_gap = Val::Px(CELL_GAP * 2.);
                                        })
                                        .item(
                                            // need to add another wrapping node here so the special output `Down`
                                            // handler doesn't overwrite the default `cell` `Down` handler
                                            El::<Node>::new()
                                            .child({
                                                let dragging_empty = SignalBuilder::from_resource::<Dragging>()
                                                    .map_in(|Dragging { item, .. }| item.is_none())
                                                    .dedupe();
                                                let output_has_item = SignalBuilder::from_component_lazy::<CellContent>(handles.craft_output.clone())
                                                    .map_in(|CellContent(x)| x.is_some())
                                                    .dedupe();
                                                let enabled = signal::and!(dragging_empty, output_has_item).dedupe();

                                                cell_with_handle(handles.craft_output.clone(), None, false)
                                                    .with_builder(|builder| {
                                                        builder
                                                            .component_signal::<OutputPressDisabled, _>(
                                                                enabled
                                                                    .clone()
                                                                    .map_in(|ok| !ok)
                                                                    .dedupe()
                                                                    .map_true_in(|| OutputPressDisabled),
                                                            )
                                                            .observe({
                                                                let inputs = handles.craft_inputs.clone();
                                                                move |press: On<Pointer<Press>>,
                                                                      disabled: Query<&OutputPressDisabled>,
                                                                      mut contents: Query<&mut CellContent>| {
                                                                    if disabled.contains(press.entity) {
                                                                        return;
                                                                    }
                                                                    for input in inputs.iter() {
                                                                        if let Ok(mut c) = contents.get_mut(input.get()) {
                                                                            c.0 = None;
                                                                        }
                                                                    }
                                                                }
                                                            })
                                                    })
                                                    .align(Align::center())
                                            })
                                        )
                                        .item(arrow())
                                        .item({
                                            El::<Node>::new().with_node(|mut node| {
                                                node.width = Val::Px(CELL_WIDTH * 2. + CELL_GAP);
                                            })
                                            .child({
                                                let craft_inputs = handles.craft_inputs.clone();
                                                let cells = (0..4).map(move |i| {
                                                    cell_with_handle(
                                                        craft_inputs[i].clone(),
                                                        bern_cell_data_option(0.2),
                                                        true,
                                                    )
                                                });
                                                Grid::<Node>::new()
                                                    .with_node(|mut node| {
                                                        node.width = Val::Percent(100.);
                                                        node.height = Val::Percent(100.);
                                                        node.column_gap = Val::Px(CELL_GAP);
                                                        node.row_gap = Val::Px(CELL_GAP);
                                                    })
                                                    .row_wrap_cell_width(CELL_WIDTH)
                                                    .cells(cells)
                                                    .align_content(Align::new().center_x())
                                            })
                                        })
                                }),
                        ),
                )
                .item(
                    El::<Node>::new()
                        .with_node(|mut node| node.width = Val::Percent(100.))
                        .child(
                            grid((0..27).map(|_| bern_cell_data_option(0.5)))
                                .align_content(Align::new().center_x()),
                        ),
                )
                .item(
                    Row::<Node>::new()
                        .with_node(|mut node| {
                            node.column_gap = Val::Px(CELL_GAP);
                        })
                        .items((0..9).map(|_| bern_cell(0.5, true))),
                ),
        )
}

fn ui_root(handles: InventoryHandles) -> impl Element {
    Stack::<Node>::new()
        .cursor_disableable_signal(CursorIcon::default(), is_dragging_signal())
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .with_builder(|builder| {
            builder
                .insert(Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                })
                .observe(|move_: On<Pointer<Move>>, mut pointer: ResMut<PointerPosition>| {
                    pointer.0 = move_.pointer_location.position;
                })
        })
        .align_content(Align::center())
        .layer(inventory_with_handles(handles.clone()))
        .layer_signal(is_dragging_signal().dedupe().map_true_in(move || {
            let index_signal = SignalBuilder::from_resource::<Dragging>()
                .map_in(|Dragging { item, .. }| item.map(|d| d.index).unwrap_or(0))
                .dedupe();
            let count_signal = SignalBuilder::from_resource::<Dragging>()
                .map_in(|Dragging { item, .. }| item.map(|d| d.count).unwrap_or(0))
                .dedupe();

            icon(index_signal, count_signal)
                .with_builder(|builder| {
                    builder.on_spawn_with_system(
                        |In(entity): In<Entity>, pointer: Res<PointerPosition>, mut nodes: Query<&mut Node>| {
                            if let Ok(mut node) = nodes.get_mut(entity) {
                                node.left = Val::Px(pointer.0.x - CELL_WIDTH / 2.);
                                node.top = Val::Px(pointer.0.y - CELL_WIDTH / 2.);
                            }
                        },
                    )
                })
                .cursor(CursorIcon::System(SystemCursorIcon::Grabbing))
                .with_node(|mut node| {
                    node.width = Val::Px(CELL_WIDTH);
                    node.height = Val::Px(CELL_WIDTH);
                    node.position_type = PositionType::Absolute;
                })
                .global_z_index(GlobalZIndex(1))
                .on_signal_with_node(pointer_position_signal(), set_dragging_position)
                .type_erase()
        }))
}

fn set_dragging_position(mut node: Mut<Node>, pointer_position: Vec2) {
    node.left = Val::Px(pointer_position.x - CELL_WIDTH / 2.);
    node.top = Val::Px(pointer_position.y - CELL_WIDTH / 2.);
}

fn register_crafting_outputter(world: &mut World, handles: InventoryHandles) -> SignalHandle {
    SignalBuilder::from_system({
        let inputs = handles.craft_inputs.clone();
        move |_: In<()>, contents: Query<&CellContent>| {
            let vals = [0usize, 1, 2, 3].map(|i| contents.get(inputs[i].get()).ok().and_then(|c| c.0));
            Some(vals)
        }
    })
    .dedupe()
    .map({
        let out = handles.craft_output.clone();
        move |In(vals): In<[Option<CellData>; 4]>, mut contents: Query<&mut CellContent>| {
            let all_filled = vals.iter().all(Option::is_some);
            if let Ok(mut output) = contents.get_mut(out.get()) {
                output.0 = all_filled.then(|| random_cell_data(&mut rand::rng()));
            }
        }
    })
    .register(world)
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum AssetState {
    #[default]
    Loading,
    Loaded,
}
