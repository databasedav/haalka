// Fixed-size grid, some spaces with items and some empty.
// Each item slot has an image of the item and the item count overlayed on the image.
// Items can be moved with drag and drop.
//     Both image and item count move along with the cursor while dragging.
//     The image and item count are not visible in the original position while dragging.
//     You can leave the bounding box of the inventory while dragging.
// A tooltip with the item's name is shown when hovering over an item.

use std::sync::OnceLock;

use bevy::{prelude::*, transform, utils::HashMap};
use bevy_asset_loader::prelude::*;
use futures_signals::signal::Mutable;
use haalka::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin,
        ))
        .add_state::<AssetState>()
        .add_loading_state(
            LoadingState::new(AssetState::Loading)
                .continue_to_state(AssetState::Loaded)
                .load_collection::<RpgIconSheet>(),
        )
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AssetState::Loaded), spawn_ui_root)
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
const CELL_LIGHT_BORDER_COLOR: Color = Color::hsl(0., 0., 0.98);

#[static_ref]
fn item_names() -> &'static HashMap<usize, &'static str> {
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
        (130, "disk (save)"),
        (131, "gear (options)"),
        (132, "check mark"),
        (133, "x mark"),
        (134, "cursor"),
        (135, "speech bubble"),
        (136, "skull (big)"),
        (137, "skull (small)"),
        (138, "heart (big)"),
        (139, "heart (small)"),
        (140, "fire"),
        (141, "water"),
        (142, "earth"),
        (143, "air"),
        (144, "fire bolt"),
        (145, "ice bolt"),
        (146, "lightning bolt"),
        (147, "buff"),
        (148, "debuff"),
        (149, "targeted"),
        (150, "sword slash"),
        (151, "sword slash (red)"),
        (152, "sword slash (yellow)"),
        (153, "sword slash (blue)"),
        (154, "thrust"),
        (155, "sword buff"),
        (156, "crush"),
        (157, "impale"),
        (158, "defense buff"),
        (159, "triple shot"),
        (160, "poison nova"),
        (161, "rage"),
        (162, "icy death"),
        (163, "haste"),
        (164, "far sight (purple)"),
        (165, "far sight (green)"),
        (166, "heal"),
        (167, "heal_2"),
        (168, "bleeding"),
        (169, "poisoned"),
        (170, "bleeding damage"),
        (171, "poison damage"),
        (172, "fire damage"),
        (173, "cold damage"),
        (174, "electric damage"),
        (175, "bleeding resistance"),
        (176, "poison resistance"),
        (177, "fire resistance"),
        (178, "cold resistance"),
        (179, "electric resistance"),
        (180, "bat wing"),
        (181, "bat wing (red)"),
        (182, "bat wing (black)"),
        (183, "mushroom (purple)"),
        (184, "mushroom (red)"),
        (185, "mushroom (yellow)"),
        (186, "mushroom (blue)"),
        (187, "ogre eye (red)"),
        (188, "ogre eye (yellow)"),
        (189, "ogre eye (purple)"),
        (190, "skeleton bone"),
        (191, "skeleton bone (dark)"),
        (192, "skeleton bone (red)"),
        (193, "slime goo (green)"),
        (194, "slime goo (yellow)"),
        (195, "slime goo (red)"),
        (196, "slime goo (blue)"),
        (197, "wolf fang"),
        (198, "wolf fang_2 "),
        (199, "wolf fang_3"),
        (200, "number_0"),
        (201, "number_1"),
        (202, "number_2"),
        (203, "number_3"),
        (204, "number_4"),
        (205, "number_5"),
        (206, "number_6"),
        (207, "number_7"),
        (208, "number_8"),
        (209, "number_9"),
        (210, "number_0 (red)"),
        (211, "number_1 (red)"),
        (212, "number_2 (red)"),
        (213, "number_3 (red)"),
        (214, "number_4 (red)"),
        (215, "number_5 (red)"),
        (216, "number_6 (red)"),
        (217, "number_7 (red)"),
        (218, "number_8 (red)"),
        (219, "number_9 (red)"),
        (220, "frame (copper)"),
        (221, "frame (silver)"),
        (222, "frame (gold)"),
        (223, "frame (bright gold)"),
        (224, "frame (green)"),
        (225, "frame (blue)"),
        (226, "frame (purple)"),
        (227, "frame (red)"),
        (228, "background"),
        (229, "catface =^･ω･^="),
        (230, "exclamation mark"),
        (231, "question mark"),
        (232, "minus"),
        (233, "plus"),
        (234, "equal"),
        (235, "multiply"),
        (236, "divide"),
        (237, "exclamation mark (red)"),
        (238, "question mark (red)"),
        (239, "minus (red)"),
        (240, "plus (red)"),
        (241, "equal (red)"),
        (242, "multiply (red)"),
        (243, "divide (red)"),
        (244, "mail"),
        (245, "scroll_closed"),
        (246, "letter_open"),
        (247, "scroll_open"),
        (248, "scrollvertical_closed"),
        (249, "scrollvertical_open"),
        (250, "scrolldiag"),
        (251, "scrolldiag (red)"),
        (252, "scrolldiag (purple)"),
        (253, "scrolldiag (green)"),
        (254, "scroll_open (blue)"),
        (255, "scroll_open (red)"),
        (256, "scroll_open (green)"),
        (257, "crystal ball"),
        (258, "pencil"),
        (259, "pen"),
        (260, "brush"),
        (261, "quill"),
        (262, "ink"),
        (263, "magnifying glass"),
    ])
}

static ICON_TEXTURE_ATLAS: OnceLock<Handle<TextureAtlas>> = OnceLock::new();

pub fn icon_texture_atlas() -> &'static Handle<TextureAtlas> {
    ICON_TEXTURE_ATLAS
        .get()
        .expect("expected ICON_TEXTURE_ATLAS to be initialized")
}

#[derive(AssetCollection, Resource)]
struct RpgIconSheet {
    #[asset(texture_atlas(tile_size_x = 48., tile_size_y = 48., columns = 10, rows = 27))]
    #[asset(image(sampler = nearest))]
    #[asset(path = "rpg_icon_sheet.png")]
    atlas: Handle<TextureAtlas>,
}

fn icon(
    index_signal: impl Signal<Item = usize> + Send + 'static,
    count_signal: impl Signal<Item = usize> + Send + 'static,
) -> Stack<NodeBundle> {
    Stack::<NodeBundle>::new()
        .layer(
            El::<AtlasImageBundle>::new()
                .texture_atlas(icon_texture_atlas().clone())
                .on_signal_with_texture_atlas_image(index_signal, |image, index| image.index = index),
        )
        .layer(
            El::<TextBundle>::new()
                .with_style(|style| style.top = Val::Px(6.))
                .align(Align::new().bottom().right())
                .text_signal(count_signal.map(|count| {
                    Text::from_section(
                        count.to_string(),
                        TextStyle {
                            font_size: 40.,
                            ..default()
                        },
                    )
                })),
        )
}

#[derive(Clone, Component)]
struct CellData {
    index: Mutable<usize>,
    count: Mutable<usize>,
}

fn cell() -> impl Element + Alignable {
    let cell_data_option = Mutable::new(Some(CellData {
        index: Mutable::new(0),
        count: Mutable::new(1),
    }));
    let hovered = Mutable::new(false);
    let original_position = Mutable::new(None);
    El::<NodeBundle>::new()
        .update_raw_el(clone!((cell_data_option, hovered) move |raw_el|
            raw_el
            .insert(Pickable::default())
            .component_signal::<On::<Pointer<Click>>>(hovered.signal().map_true(move ||
                On::<Pointer<Click>>::run(clone!((cell_data_option => self_cell_data_option) move |click: Listener<Pointer<Click>>| {
                    let mut consume = false;
                    if let Some(dragging_cell_data_option) = &*dragging_option().lock_ref() {
                        if self_cell_data_option.lock_ref().is_none() {
                            if let Some(dragging_cell_data) = &*dragging_cell_data_option.lock_ref() {
                                self_cell_data_option.set(Some(CellData {
                                    index: Mutable::new(dragging_cell_data.index.get()),
                                    count: Mutable::new(0),
                                }));
                            }
                        }
                        if self_cell_data_option.lock_ref().as_ref().map(|cell_data| cell_data.index.get()) == dragging_cell_data_option.lock_ref().as_ref().map(|cell_data| cell_data.index.get()) {
                            if let Some((dragging_cell_data, self_cell_data)) = dragging_cell_data_option.lock_ref().as_ref().zip(self_cell_data_option.lock_ref().as_ref()) {
                                let to_add = {
                                    if matches!(click.button, PointerButton::Secondary) {
                                        *dragging_cell_data.count.lock_mut() -= 1;
                                        if dragging_cell_data.count.get() == 0 {
                                            consume = true;
                                        }
                                        1
                                    } else {
                                        let count = dragging_cell_data.count.take();
                                        consume = true;
                                        count
                                    }
                                };
                                self_cell_data.count.update(|count| count + to_add);
                            }
                        }
                    }
                    if consume {
                        if let Some(cell_data_option) = dragging_option().take() {
                            cell_data_option.take();
                        }
                    }
                }))
            ))
        ))
        .hovered_sync(hovered.clone())
        .with_style(|style| {
            style.width = Val::Px(CELL_WIDTH);
            style.height = Val::Px(CELL_WIDTH);
            style.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
        })
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| CELL_HIGHLIGHT_COLOR.into(), || CELL_BACKGROUND_COLOR.into()),
        )
        .border_color(CELL_DARK_BORDER_COLOR.into())
        .child_signal(
            cell_data_option
                .signal_cloned()
                .map_some(move |cell_data| {
                    Stack::<NodeBundle>::new()
                    .layer(
                        icon(cell_data.index.signal(), cell_data.count.signal())
                        .update_raw_el(clone!((cell_data_option) move |raw_el| {
                            raw_el
                            .insert(Pickable::default())
                            .component_signal::<On::<Pointer<Down>>>(dragging_option().signal_ref(Option::is_some).map_false(clone!((cell_data_option) move ||
                                On::<Pointer<Down>>::run(clone!((cell_data_option) move |down: Listener<Pointer<Down>>| {
                                    dragging_option().set(Some(Mutable::new(cell_data_option.take())));
                                    pointer_position().set(down.pointer_location.position.into());
                                }))
                            )))
                        }))
                    )
                    .layer_signal(
                        signal::and(hovered.signal(), dragging_option().signal_ref(Option::is_none)).dedupe()
                        .map_true(clone!((original_position) move ||
                            El::<NodeBundle>::new()
                                // TODO: global transform isn't populated on spawn
                                // .with_global_transform(clone!((original_position) move |transform| original_position.set(Some(transform.compute_transform().translation.xy()))))
                                .with_style(|style| {
                                    style.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
                                    style.position_type = PositionType::Absolute;
                                    style.height = Val::Px(CELL_WIDTH);
                                    style.max_width = Val::Px(CELL_WIDTH * 3.);
                                })
                                .update_raw_el(clone!((original_position) move |raw_el| {
                                    raw_el
                                    .on_signal_with_entity(pointer_position().signal(), move |entity, (mut left, mut top)| {
                                        if let Some(transform) = entity.get::<GlobalTransform>() {
                                            // TODO: global transform isn't populated on spawn so we have to set it here
                                            if original_position.get().is_none() {
                                                original_position.set(Some(transform.compute_transform().translation.xy()));
                                            }
                                            left -= original_position.get().unwrap().x - CELL_WIDTH / 2.;
                                            top -= original_position.get().unwrap().y + CELL_WIDTH / 2.;
                                        }
                                        if let Some(mut style) = entity.get_mut::<Style>() {
                                            style.left = Val::Px(left);
                                            style.top = Val::Px(top);
                                        }
                                    })
                                }))
                                .z_index(ZIndex::Global(1))
                                .background_color(CELL_BACKGROUND_COLOR.into())
                                .border_color(CELL_DARK_BORDER_COLOR.into())
                                .child(
                                    El::<TextBundle>::new()
                                    .with_style(|style| style.position_type = PositionType::Absolute)
                                    .text_signal(
                                        cell_data.index.signal()
                                        .map(|i|
                                            Text::from_section(
                                                item_names().get(&i).unwrap().to_string(),
                                                TextStyle { font_size: 50., ..default() }
                                            )
                                        )
                                    )
                                )
                        ))
                    )
                })
        )
}

fn grid(n: usize) -> Grid<NodeBundle> {
    Grid::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
            style.column_gap = Val::Px(CELL_GAP);
            style.row_gap = Val::Px(CELL_GAP);
        })
        .row_wrap_cell_width(CELL_WIDTH)
        .cells((0..n).into_iter().map(|_| cell()))
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn dot() -> impl Element {
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Px(CELL_BORDER_WIDTH * 2.);
            style.height = Val::Px(CELL_BORDER_WIDTH * 2.);
        })
        .background_color(CELL_BACKGROUND_COLOR.into())
}

fn dot_row(n: usize) -> impl Element {
    Row::<NodeBundle>::new().items((0..n).into_iter().map(|_| dot()))
}

fn arrow() -> impl Element {
    Column::<NodeBundle>::new()
        .align_content(Align::center())
        .items((0..=6).into_iter().map(|i| dot_row(2 * i + 1)))
        .items((0..6).into_iter().map(|_| dot_row(3)))
}

fn inventory() -> impl Element {
    El::<NodeBundle>::new()
        .align(Align::center())
        .with_style(|style| {
            style.height = Val::Px(INVENTORY_SIZE);
            style.width = Val::Px(INVENTORY_SIZE);
        })
        .child(
            Column::<NodeBundle>::new()
                .with_style(|style| {
                    style.height = Val::Percent(100.);
                    style.width = Val::Percent(100.);
                    style.row_gap = Val::Px(CELL_GAP * 4.);
                })
                .background_color(INVENTORY_BACKGROUND_COLOR.into())
                .align_content(Align::center())
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|style| {
                            style.column_gap = Val::Px(CELL_GAP);
                            style.width = Val::Percent(100.);
                        })
                        .item(
                            Row::<NodeBundle>::new()
                                .align_content(Align::center())
                                .with_style(|style| {
                                    style.column_gap = Val::Px(CELL_GAP);
                                    style.width = Val::Percent(60.);
                                    style.padding = UiRect::horizontal(Val::Px(CELL_GAP * 3.));
                                })
                                .item(
                                    Column::<NodeBundle>::new()
                                        .with_style(|style| style.row_gap = Val::Px(CELL_GAP))
                                        .items((0..4).into_iter().map(|_| cell())),
                                )
                                .item(
                                    El::<NodeBundle>::new()
                                        .with_style(|style| {
                                            style.height = Val::Px(CELL_WIDTH * 4. + CELL_GAP * 3.);
                                            style.width = Val::Percent(100.);
                                        })
                                        .background_color(Color::BLACK.into()),
                                )
                                .item(
                                    Column::<NodeBundle>::new()
                                        .with_style(|style| style.row_gap = Val::Px(CELL_GAP))
                                        .items((0..4).into_iter().map(|_| cell())),
                                ),
                        )
                        .item(
                            El::<NodeBundle>::new()
                                .with_style(|style| {
                                    style.width = Val::Percent(40.);
                                    style.height = Val::Percent(100.);
                                })
                                .align_content(Align::center())
                                .child(
                                    Column::<NodeBundle>::new()
                                        .with_style(|style| {
                                            style.row_gap = Val::Px(CELL_GAP * 2.);
                                        })
                                        .item(cell().align(Align::center()))
                                        .item(arrow())
                                        .item(
                                            El::<NodeBundle>::new()
                                                .with_style(|style| style.width = Val::Px(CELL_WIDTH * 2. + CELL_GAP))
                                                .child(grid(4).align_content(Align::new().center_x())),
                                        ),
                                ),
                        ),
                )
                .item(
                    El::<NodeBundle>::new()
                        .with_style(|style| style.width = Val::Percent(100.))
                        .child(grid(27).align_content(Align::new().center_x())),
                )
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|style| {
                            style.column_gap = Val::Px(CELL_GAP);
                        })
                        .items((0..9).into_iter().map(|_| cell())),
                ),
        )
}

#[static_ref]
fn dragging_option() -> &'static Mutable<Option<Mutable<Option<CellData>>>> {
    Mutable::new(None)
}

#[static_ref]
fn pointer_position() -> &'static Mutable<(f32, f32)> {
    Mutable::new(default())
}

fn spawn_ui_root(world: &mut World) {
    ICON_TEXTURE_ATLAS
        .set(
            world
                .get_resource::<RpgIconSheet>()
                .expect("expected rpg icon sheet to be loaded")
                .atlas
                .clone(),
        )
        .expect("failed to initialize ICON_TEXTURE_ATLAS");
    Stack::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
        })
        .update_raw_el(|raw_el| {
            raw_el
                .insert(On::<Pointer<Move>>::run(|move_: Listener<Pointer<Move>>| {
                    pointer_position().set(move_.pointer_location.position.into());
                }))
                .component_signal::<Pickable>(
                    dragging_option()
                        .signal_ref(Option::is_some)
                        .map_true(|| Pickable::default()),
                )
        })
        .align_content(Align::center())
        .layer(inventory())
        .layer_signal(
            dragging_option()
                .signal_cloned()
                .map_some(|cell_data_option| cell_data_option.signal_cloned())
                .map(signal::option)
                .flatten()
                .map(Option::flatten)
                .map_some(move |cell_data| {
                    icon(cell_data.index.signal(), cell_data.count.signal())
                        .with_style(move |style| {
                            style.position_type = PositionType::Absolute;
                            style.width = Val::Px(CELL_WIDTH);
                            style.height = Val::Px(CELL_WIDTH);
                        })
                        .z_index(ZIndex::Global(1))
                        .on_signal_with_style(pointer_position().signal(), move |style, pointer_position| {
                            style.left = Val::Px(pointer_position.0 - CELL_WIDTH / 2.);
                            style.top = Val::Px(pointer_position.1 - CELL_WIDTH / 2.);
                        })
                }),
        )
        .spawn(world);
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum AssetState {
    #[default]
    Loading,
    Loaded,
}
