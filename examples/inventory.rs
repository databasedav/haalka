//! - Fixed-size grid, some spaces with items and some empty.
//! - Each item slot has an image of the item and the item count overlayed on the image.
//! - Items can be moved with drag and drop.
//!   - Both image and item count move along with the cursor while dragging.
//!   - The image and item count are not visible in the original position while dragging.
//!   - You can leave the bounding box of the inventory while dragging.
//! - A tooltip with the item's name is shown when hovering over an item.

mod utils;
use utils::*;

use bevy::{
    platform::{collections::HashMap, sync::LazyLock},
    prelude::*,
};
use bevy_asset_loader::prelude::*;
use bevy_rand::prelude::*;
use haalka::prelude::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<WyRand>::default()))
        .init_state::<AssetState>()
        .add_loading_state(
            LoadingState::new(AssetState::Loading)
                .continue_to_state(AssetState::Loaded)
                .load_collection::<RpgIconSheet>(),
        )
        .insert_resource(PointerPosition::default())
        // .add_systems(Startup, character_camera)
        // .add_systems(Startup, setup_3d)
        // .add_systems(Update, rotate_prism)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Camera2d, IsDefaultUiCamera));
        })
        .add_systems(
            OnEnter(AssetState::Loaded),
            (|world: &mut World| {
                ui_root().spawn(world);
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
                .with_builder(|builder| {
                    builder.on_spawn_with_system(
                        |In(entity): In<_>, rpg_icon_sheet: Res<RpgIconSheet>, mut commands: Commands| {
                            if let Ok(mut entity) = commands.get_entity(entity) {
                                entity.insert(ImageNode {
                                    image: rpg_icon_sheet.image.clone(),
                                    texture_atlas: Some(TextureAtlas::from(rpg_icon_sheet.layout.clone())),
                                    ..default()
                                });
                            }
                        },
                    )
                })
                .on_signal_with_image_node(index_signal.dedupe(), |mut image_node: Mut<ImageNode>, index| {
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
                .text_signal(
                    count_signal
                        .dedupe()
                        .map_in_ref(ToString::to_string)
                        .map_in(Text)
                        .map_in(Some),
                ),
        )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellData {
    index: usize,
    count: usize,
}

#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
struct CellContent(Option<CellData>);

#[derive(Resource, Clone, Copy, Debug, Default, Deref)]
struct PointerPosition(Vec2);

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
struct Dragging {
    item: CellData,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
struct CraftOutputClearGuard;

#[derive(Component, Clone)]
struct BlockClick;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CraftInputCell;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CraftOutputSlot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CraftOutputAction {
    Generate,
    Clear,
    SkipClear,
}

#[derive(Component, Clone)]
struct PressHandlingDisabled;

#[derive(Component, Default)]
struct TooltipOrigin(Option<Vec2>);

fn is_dragging() -> impl Signal<Item = bool> + Clone {
    signal::from_system(|_: In<()>, dragging: Option<Res<Dragging>>| dragging.is_some()).dedupe()
}

fn pointer_position_signal() -> impl Signal<Item = Vec2> + Clone {
    signal::from_resource_changed::<PointerPosition>().map_in(deref_copied)
}

fn cell(insertable: bool) -> impl Element + BuilderPassThrough + PointerEventAware {
    let lazy_entity = LazyEntity::new();
    let dragging = is_dragging();
    let hovered = signal::from_entity(lazy_entity.clone())
        .has_component::<Hovered>()
        .dedupe();
    let content = signal::from_component_changed::<CellContent>(lazy_entity.clone());
    let populated = content.clone().map_in(|CellContent(x)| x.is_some()).dedupe();

    {
        let el = El::<Node>::new()
            .lazy_entity(lazy_entity.clone())
            .insert((Pickable::default(), Hoverable))
            .insert(CellContent::default())
            .with_builder(|builder| {
                builder.component_signal(
                    signal::any!(dragging.clone(), populated.clone().not())
                        .dedupe()
                        .map_true_in(|| PressHandlingDisabled),
                )
            })
            .on_hovered_change(
                |In((entity, hover_data)): In<(Entity, HoverData)>, mut commands: Commands| {
                    if !hover_data.hovered {
                        commands.entity(entity).remove::<BlockClick>();
                    }
                },
            )
            .on_pressed_change(
                |In((entity, press_data)): In<(Entity, PressData)>,
                 disabled: Query<&PressHandlingDisabled>,
                 output_slots: Query<(), With<CraftOutputSlot>>,
                 mut commands: Commands,
                 mut pointer: ResMut<PointerPosition>,
                 mut contents: Query<&mut CellContent>| {
                    if press_data.pressed {
                        if disabled.contains(entity) {
                            return;
                        }

                        commands.entity(entity).insert(BlockClick);

                        let mut to_drag: Option<CellData> = None;
                        if let Ok(mut content) = contents.get_mut(entity) {
                            match press_data.button {
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
                                    to_drag = content.take();
                                }
                            }

                            if matches!(press_data.button, PointerButton::Secondary)
                                && output_slots.contains(entity)
                                && content.0.is_some()
                            {
                                commands.insert_resource(CraftOutputClearGuard);
                            }
                        }

                        if let Some(item) = to_drag {
                            commands.insert_resource(Dragging { item });
                        }
                        let pos = press_data.pointer_location.position;
                        pointer.0 = pos;
                    } else {
                        commands.entity(entity).remove::<BlockClick>();
                    }
                },
            );

        if insertable {
            el.on_click_disableable::<BlockClick, _>(
                |In((entity, click)): In<(Entity, Pointer<Click>)>,
                 dragging: Option<ResMut<Dragging>>,
                 mut commands: Commands,
                 mut contents: Query<&mut CellContent>| {
                    let Some(mut dragging) = dragging else {
                        return;
                    };
                    let mut dragged = dragging.item;

                    let Ok(mut content) = contents.get_mut(entity) else {
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
                                if dragged.count > 0 {
                                    dragging.item = dragged;
                                } else {
                                    commands.remove_resource::<Dragging>();
                                }
                            } else {
                                // Drop entire stack into empty slot.
                                content.0 = Some(dragged);
                                commands.remove_resource::<Dragging>();
                            }
                        }
                        Some(existing) => {
                            if existing.index == dragged.index {
                                if matches!(click.button, PointerButton::Secondary) {
                                    existing.count = existing.count.saturating_add(1);
                                    dragged.count = dragged.count.saturating_sub(1);
                                    if dragged.count > 0 {
                                        dragging.item = dragged;
                                    } else {
                                        commands.remove_resource::<Dragging>();
                                    }
                                } else {
                                    existing.count = existing.count.saturating_add(dragged.count);
                                    commands.remove_resource::<Dragging>();
                                }
                            } else {
                                // Swap different items.
                                let tmp = *existing;
                                *existing = dragged;
                                dragging.item = tmp;
                            }
                        }
                    }
                },
            )
        } else {
            el
        }
    }
    .cursor_disableable_signal(
        CursorIcon::System(SystemCursorIcon::Grab),
        signal::any!(populated.clone().not(), dragging.clone()).dedupe(),
    )
    .with_node(|mut node| {
        node.width = Val::Px(CELL_WIDTH);
        node.height = Val::Px(CELL_WIDTH);
        node.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
    })
    .background_color_signal(
        hovered
            .clone()
            .map_bool_in(|| CELL_HIGHLIGHT_COLOR, || CELL_BACKGROUND_COLOR)
            .map_in(BackgroundColor)
            .map_in(Some),
    )
    .border_color(BorderColor::all(CELL_DARK_BORDER_COLOR))
    .child({
        let index = content
            .clone()
            .map_in(|CellContent(cell_data_option)| cell_data_option.map(|cell_data| cell_data.index).unwrap_or(0))
            .dedupe();
        let count = content
            .clone()
            .map_in(|CellContent(cell_data_option)| cell_data_option.map(|cell_data| cell_data.count).unwrap_or(0))
            .dedupe();
        icon(index, count)
            .with_builder(|builder| {
                builder.component_signal(
                    populated
                        .clone()
                        .map_bool_in(|| Visibility::Inherited, || Visibility::Hidden)
                        .map_in(Some),
                )
            })
            .layer_signal(
                signal::all!(hovered.clone(), populated.clone(), dragging.clone().not())
                    .dedupe()
                    .map_true_in(move || tooltip(lazy_entity.clone())),
            )
    })
}

fn tooltip(cell_entity: LazyEntity) -> impl Element + Clone {
    let tooltip_entity = LazyEntity::new();
    El::<Node>::new()
        .lazy_entity(tooltip_entity.clone())
        .insert(TooltipOrigin::default())
        .with_node(|mut node| {
            node.height = Val::Px(CELL_WIDTH);
            node.position_type = PositionType::Absolute;
            node.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
            node.padding = UiRect::horizontal(Val::Px(10.));
        })
        .visibility(Visibility::Hidden)
        .with_builder(|builder| {
            builder.on_signal_with_entity(pointer_position_signal(), move |mut entity, pointer| {
                // Initialize the tooltip's world-space origin once GlobalTransform is available.
                let origin = if let (Some(transform), Some(mut origin)) = (
                    entity.get::<UiGlobalTransform>().cloned(),
                    entity.get_mut::<TooltipOrigin>(),
                ) {
                    if origin.0.is_none() {
                        origin.0 = Some(transform.translation.xy());
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
                        entity.insert(Visibility::Visible);
                    }
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
                    signal::from_component_changed::<CellContent>(cell_entity)
                        .map_in(|CellContent(cell_data_option)| cell_data_option.map(|cell_data| cell_data.index))
                        .dedupe()
                        .map_some_in(|index| ITEM_NAMES.get(&index).copied())
                        .map_in(Option::flatten)
                        .map_some_in(Into::into)
                        .map_some_in(Text),
                ),
        )
}

fn random_cell_data(rng: &mut impl Rng) -> CellData {
    CellData {
        index: rng.random_range(0..ITEM_NAMES.len()),
        count: rng.random_range(1..=64),
    }
}

fn random_cell(probability: f64, insertable: bool) -> impl Element {
    cell(insertable).with_builder(move |builder| {
        builder.on_spawn_with_system(
            move |In(entity): In<Entity>,
                  mut rng: Single<&mut WyRand, With<GlobalRng>>,
                  mut contents: Query<&mut CellContent>| {
                if rng.random_bool(probability) {
                    if let Ok(mut content) = contents.get_mut(entity) {
                        content.0 = Some(random_cell_data(rng.as_mut()));
                    }
                }
            },
        )
    })
}

fn random_grid<F>(n: usize, probability: f64, on_spawn_cell_option: Option<F>) -> impl Element
where
    F: Fn(&mut World, Entity) + Clone + Send + Sync + 'static,
{
    Grid::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
            node.column_gap = Val::Px(CELL_GAP);
            node.row_gap = Val::Px(CELL_GAP);
        })
        .row_wrap_cell_width(CELL_WIDTH)
        .cells((0..n).map(move |_| {
            let on_spawn_cell = on_spawn_cell_option.clone();
            random_cell(probability, true).with_builder(move |builder| {
                if let Some(on_spawn_cell) = on_spawn_cell.clone() {
                    builder.on_spawn(on_spawn_cell)
                } else {
                    builder
                }
            })
        }))
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

fn craft_output_cell() -> impl Element {
    let output_action = signal::from_system(
        |_: In<()>,
         mut prev_filled: Local<Option<bool>>,
         changed: Query<(), (With<CraftInputCell>, Changed<CellContent>)>,
         input_cells: Query<&CellContent, With<CraftInputCell>>,
         guard: Option<Res<CraftOutputClearGuard>>| {
            let changed = !changed.is_empty();
            let filled = input_cells.iter().all(|c| c.0.is_some());
            let was_filled = prev_filled.unwrap_or(filled);

            *prev_filled = Some(filled);

            if filled {
                changed.then_some(CraftOutputAction::Generate)
            } else if was_filled {
                if guard.is_some() {
                    Some(CraftOutputAction::SkipClear)
                } else {
                    Some(CraftOutputAction::Clear)
                }
            } else {
                None
            }
        },
    )
    .dedupe();
    let outputter = output_action
        .map_some(
            |In(action): In<CraftOutputAction>,
             mut commands: Commands,
             mut rng: Single<&mut WyRand, With<GlobalRng>>,
             mut output: Single<&mut CellContent, With<CraftOutputSlot>>| {
                match action {
                    CraftOutputAction::Generate => {
                        output.0 = Some(random_cell_data(rng.as_mut()));
                    }
                    CraftOutputAction::Clear => {
                        output.0 = None;
                    }
                    CraftOutputAction::SkipClear => {
                        commands.remove_resource::<CraftOutputClearGuard>();
                    }
                }
            },
        )
        .task();
    let output_empty =
        signal::from_system(|_: In<()>, output: Single<&CellContent, With<CraftOutputSlot>>| output.is_none()).dedupe();
    let press_disabled = signal::any!(is_dragging(), output_empty.clone()).dedupe();
    cell(false)
        .insert(CraftOutputSlot)
        .with_builder(|builder| builder.hold_tasks([outputter]))
        .on_pressed_change_disableable_signal(
            |In((_entity, press)): In<(Entity, PressData)>,
             input_cells: Query<Entity, With<CraftInputCell>>,
             mut contents: Query<&mut CellContent>| {
                if !press.pressed {
                    return;
                }

                let inputs_full = input_cells
                    .iter()
                    .take(4)
                    .all(|entity| contents.get(entity).map(|c| c.0.is_some()).unwrap_or(false));
                if !inputs_full {
                    return;
                }

                for entity in input_cells.iter().take(4) {
                    if let Ok(mut content) = contents.get_mut(entity) {
                        content.0 = None;
                    }
                }
            },
            press_disabled,
        )
}

fn side_column() -> impl Element {
    Column::<Node>::new()
        .with_node(|mut node| node.row_gap = Val::Px(CELL_GAP))
        .items((0..4).map(|_| random_cell(0.5, true)))
}

fn character_preview() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.height = Val::Px(CELL_WIDTH * 4. + CELL_GAP * 3.);
            node.width = Val::Percent(100.);
        })
        .background_color(BackgroundColor(Color::BLACK))
}

fn equipment_section() -> impl Element {
    Row::<Node>::new()
        .align_content(Align::center())
        .with_node(|mut node| {
            node.width = Val::Percent(60.);
            node.column_gap = Val::Px(CELL_GAP);
            node.padding = UiRect::horizontal(Val::Px(CELL_GAP * 3.));
        })
        .item(side_column())
        .item(character_preview())
        .item(side_column())
}

fn craft_input_grid() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Px(CELL_WIDTH * 2. + CELL_GAP);
            node.height = Val::Px(CELL_WIDTH * 2. + CELL_GAP);
        })
        .child(random_grid(
            4,
            0.2,
            Some(|world: &mut World, entity: Entity| {
                if let Ok(mut entity) = world.get_entity_mut(entity) {
                    entity.insert(CraftInputCell);
                }
            }),
        ))
}

fn crafting_section() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(40.);
            node.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        .child(
            Column::<Node>::new()
                .with_node(|mut node| {
                    node.row_gap = Val::Px(CELL_GAP * 2.);
                })
                .item(craft_output_cell().align(Align::center()))
                .item(arrow())
                .item(craft_input_grid()),
        )
}

fn top_row() -> impl Element {
    Row::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.column_gap = Val::Px(CELL_GAP);
        })
        .item(equipment_section())
        .item(crafting_section())
}

fn main_grid() -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| node.width = Val::Percent(100.))
        .child(random_grid::<fn(&mut World, Entity)>(27, 0.5, None).align_content(Align::new().center_x()))
}

fn hotbar() -> impl Element {
    Row::<Node>::new()
        .with_node(|mut node| {
            node.column_gap = Val::Px(CELL_GAP);
        })
        .items((0..9).map(|_| random_cell(0.5, true)))
}

fn inventory() -> impl Element {
    El::<Node>::new()
        .align_content(Align::center())
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
                .item(top_row())
                .item(main_grid())
                .item(hotbar()),
        )
}

fn ui_root() -> impl Element {
    Stack::<Node>::new()
        .cursor_disableable_signal(CursorIcon::default(), is_dragging())
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .with_builder(move |builder| {
            builder.on_spawn_with_system(
                move |In(entity): In<_>, camera: Single<Entity, With<IsDefaultUiCamera>>, mut commands: Commands| {
                    // https://github.com/bevyengine/bevy/discussions/11223
                    if let Ok(mut commands) = commands.get_entity(entity) {
                        commands.try_insert(UiTargetCamera(*camera));
                    }
                },
            )
        })
        .insert(Pickable::default())
        .observe(|move_: On<Pointer<Move>>, mut pointer: ResMut<PointerPosition>| {
            pointer.0 = move_.pointer_location.position;
        })
        .align_content(Align::center())
        .layer(inventory())
        // dragging icon
        .layer_signal(is_dragging().dedupe().map_true_in(move || {
            let index = signal::from_system(|_: In<()>, dragging: Option<Res<Dragging>>| {
                dragging.map(|d| d.item.index).unwrap_or(0)
            })
            .dedupe();
            let count = signal::from_system(|_: In<()>, dragging: Option<Res<Dragging>>| {
                dragging.map(|d| d.item.count).unwrap_or(0)
            })
            .dedupe();
            icon(index, count)
                .insert(Pickable {
                    should_block_lower: false, // allows triggering hover states for cells below dragging icon
                    ..Pickable::default()
                })
                .cursor(CursorIcon::System(SystemCursorIcon::Grabbing))
                .with_node(|mut node| {
                    node.width = Val::Px(CELL_WIDTH);
                    node.height = Val::Px(CELL_WIDTH);
                    node.position_type = PositionType::Absolute;
                })
                .global_z_index(GlobalZIndex(1))
                .on_signal_with_node(pointer_position_signal(), set_dragging_position)
        }))
}

fn set_dragging_position(mut node: Mut<Node>, pointer_position: Vec2) {
    node.left = Val::Px(pointer_position.x - CELL_WIDTH / 2.);
    node.top = Val::Px(pointer_position.y - CELL_WIDTH / 2.);
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum AssetState {
    #[default]
    Loading,
    Loaded,
}
