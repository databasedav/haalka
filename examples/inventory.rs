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

use std::{collections::HashMap, convert::identity, sync::OnceLock};

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_mod_picking::{
    events::{Click, Down, Move, Pointer, Up},
    prelude::{Pickable, PointerButton},
};
use haalka::{prelude::*, raw::DeferredUpdaterAppendDirection};
use rand::{
    distributions::{Bernoulli, Distribution},
    Rng,
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
        .add_systems(Startup, camera)
        .add_systems(
            OnEnter(AssetState::Loaded),
            (set_icon_texture_atlas, |world: &mut World| {
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

static ITEM_NAMES: Lazy<HashMap<usize, &'static str>> = Lazy::new(|| {
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
    #[asset(image(sampler = nearest))]
    #[asset(path = "rpg_icon_sheet.png")]
    image: Handle<Image>,
}

fn icon(
    index_signal: impl Signal<Item = usize> + Send + 'static,
    count_signal: impl Signal<Item = usize> + Send + 'static,
) -> Stack<NodeBundle> {
    Stack::new()
        .layer(
            El::<ImageBundle>::new()
                .image(UiImage::from(icon_sheet().image.clone()))
                .update_raw_el(|raw_el| {
                    raw_el
                        .insert(TextureAtlas::from(icon_sheet().layout.clone()))
                        .on_signal_with_component(index_signal, |mut texture_atlas: Mut<TextureAtlas>, index| {
                            texture_atlas.index = index;
                        })
                }),
        )
        .layer(
            El::<TextBundle>::new()
                .with_style(|mut style| style.top = Val::Px(6.))
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

fn cell(cell_data_option: Mutable<Option<CellData>>, insertable: bool) -> impl Element {
    let hovered = Mutable::new(false);
    let original_position: Mutable<Option<Vec2>> = Mutable::new(None);
    let down = Mutable::new(false);
    // let stop_propagation_trigger = Mutable::new(false);
    // let cursor_disabling_forwarder = spawn(sync_neq(
    //     signal::or(cell_data_option.signal_ref(Option::is_none), is_dragging()),
    //     stop_propagation_trigger.clone(),
    // ));
    El::<NodeBundle>::new()
        .update_raw_el(clone!((cell_data_option, hovered, down/* , stop_propagation_trigger */) move |mut raw_el| {
            // raw_el = raw_el.hold_tasks([cursor_disabling_forwarder]);
            if insertable {
                raw_el = raw_el
                .insert((
                    Pickable::default(),
                    On::<Pointer<Up>>::run(clone!((down) move || down.set_neq(false))),
                ))
                // `.component_signal` conveniently allows us to reactively add/remove components,
                // if the provided signal returns `None`, then the component is removed; but the
                // signal below doesn't look like it returns an `Option`? actually it does thanks to
                // `.map_true` which is syntactic sugar for `.map(|bool| if bool { Some(...) } else { None }))`
                .component_signal::<On::<Pointer<Click>>, _>(
                    // we don't want the click listener to trigger if we've just grabbed some of
                    // the stack as it would immediately drop one down, so we track the `Down` state
                    signal::and(signal::not(down.signal()), hovered.signal()).dedupe()
                    .map_true(clone!((cell_data_option) move || {
                        On::<Pointer<Click>>::run(clone!((cell_data_option => self_cell_data_option/* , stop_propagation_trigger */) move |click: Listener<Pointer<Click>>/* , mut commands: Commands */| {
                            let mut consume = false;
                            if let Some(dragging_cell_data_option) = &*DRAGGING_OPTION.lock_ref() {
                                if self_cell_data_option.lock_ref().is_none() {
                                    if let Some(dragging_cell_data) = &*dragging_cell_data_option.lock_ref() {
                                        self_cell_data_option.set(Some(CellData {
                                            index: Mutable::new(dragging_cell_data.index.get()),
                                            count: Mutable::new(0),
                                        }));
                                    }
                                }
                                if let Some((dragging_cell_data, self_cell_data)) = dragging_cell_data_option.lock_ref().as_ref().zip(self_cell_data_option.lock_ref().as_ref()) {
                                    if self_cell_data.index.get() == dragging_cell_data.index.get() {
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
                                    } else {
                                        self_cell_data.index.swap(&dragging_cell_data.index);
                                        self_cell_data.count.swap(&dragging_cell_data.count);
                                    }
                                }
                            }
                            if consume {
                                // because propagation stoppage may not synchronize before the `Over` event is fired,
                                // the `Over` event may leak through to the root, racing the root and cell's cursors;
                                // to avoid this we first notify the cell to stop propagation, and wait for that to
                                // propagate, before clearing the dragging cell data, which fires the `Over` event
                                //
                                // TODO: how can i address this more ergonomically? do bubbling observers help?
                                // let waiter = stop_propagation_trigger.signal().wait_for(true);
                                // async {
                                //     waiter.await;
                                    if let Some(cell_data_option) = DRAGGING_OPTION.take() {
                                        cell_data_option.take();
                                        // commands.entity(click.listener()).remove::<CursorDisabled>();
                                    }
                                // }
                                // .apply(spawn)
                                // .detach();
                                // stop_propagation_trigger.set_neq(true);
                            }
                        }))
                    }))
                );
            }
            raw_el
            .component_signal::<On::<Pointer<Down>>, _>(
                signal::and(signal::not(is_dragging()), cell_data_option.signal_ref(Option::is_some)).dedupe()
                .map_true(clone!((cell_data_option, down) move ||
                    On::<Pointer<Down>>::run(clone!((cell_data_option, down) move |pointer_down: Listener<Pointer<Down>>| {
                        let to_drag_option = {
                            if pointer_down.button == PointerButton::Secondary {
                                if let Some(cell_data) = &*cell_data_option.lock_ref() {
                                    let to_take = (cell_data.count.get() / 2).max(1);
                                    cell_data.count.update(|count| count - to_take);
                                    Some(CellData {
                                        index: Mutable::new(cell_data.index.get()),
                                        count: Mutable::new(to_take),
                                    })
                                } else {
                                    None
                                }
                            } else {
                                cell_data_option.take()
                            }
                        };
                        if cell_data_option.lock_ref().as_ref().map(|cell_data| cell_data.count.get() == 0).unwrap_or(false) {
                            cell_data_option.take();
                        }
                        DRAGGING_OPTION.set(Some(Mutable::new(to_drag_option)));
                        POINTER_POSITION.set(pointer_down.pointer_location.position.into());
                        down.set_neq(true);
                    }))
                ))
            )
        }))
        // alternative to the stop propagation trigger pattern, which is kinda/pretty cringe
        .cursor_signal(
            map_ref! {
                let &populated = cell_data_option.signal_ref(Option::is_some),
                let &is_dragging = is_dragging() => {
                    if is_dragging {
                        CursorIcon::Grabbing
                    } else if populated {
                        CursorIcon::Grab
                    } else {
                        CursorIcon::Default
                    }
                }
            }
        )
        // .cursor_disableable_signal(CursorIcon::Grab, stop_propagation_trigger.signal())
        // TODO: this is more idiomatic and should work, but it doesn't due to various shenanigans, see stop_propagation_trigger strat attempt for some exploration
        // .cursor_disableable_signal(CursorIcon::Grab, signal::or(cell_data_option.signal_ref(Option::is_none), is_dragging()))
        .hovered_sync(hovered.clone())
        .width(Val::Px(CELL_WIDTH))
        .height(Val::Px(CELL_WIDTH))
        .with_style(|mut style| style.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH)))
        .background_color_signal(
            hovered.signal()
                .map_bool(|| CELL_HIGHLIGHT_COLOR, || CELL_BACKGROUND_COLOR).map(Into::into),
        )
        .border_color(BorderColor(CELL_DARK_BORDER_COLOR))
        .child_signal(
            cell_data_option
                .signal_cloned()
                .map_some(move |cell_data| {
                    Stack::<NodeBundle>::new()
                    .layer(icon(cell_data.index.signal(), cell_data.count.signal()))
                    .layer_signal(
                        signal::and(hovered.signal(), signal::not(is_dragging())).dedupe()
                        .map_true(clone!((original_position) move || {
                            El::<NodeBundle>::new()
                                // TODO: global transform isn't populated on spawn
                                // .with_global_transform(clone!((original_position) move |transform| original_position.set(Some(transform.compute_transform().translation.xy()))))
                                .height(Val::Px(CELL_WIDTH))
                                .with_style(|mut style| {
                                    style.position_type = PositionType::Absolute;
                                    style.border = UiRect::all(Val::Px(CELL_BORDER_WIDTH));
                                    style.padding = UiRect::horizontal(Val::Px(10.));
                                })
                                .visibility(Visibility::Hidden)
                                .update_raw_el(clone!((original_position) move |raw_el| {
                                    raw_el
                                    .on_signal_with_entity(POINTER_POSITION.signal(), move |mut entity, (mut left, mut top)| {
                                        if let Some(transform) = entity.get::<GlobalTransform>() {
                                            // TODO: global transform isn't populated on spawn so we have to set it here
                                            if original_position.get().is_none() {
                                                original_position.set(Some(transform.compute_transform().translation.xy()));
                                            }
                                            let original_position = original_position.get().unwrap();
                                            left -= original_position.x - CELL_WIDTH / 2.;
                                            top -= original_position.y + CELL_WIDTH / 2.;
                                            // this fixes grey flash when inserting into an empty cell, which is caused by the item tooltip flashing on top before the frame it is moved
                                            entity.insert(Visibility::Visible);
                                        }
                                        if let Some(mut style) = entity.get_mut::<Style>() {
                                            style.left = Val::Px(left);
                                            style.top = Val::Px(top);
                                        }
                                    })
                                }))
                                .z_index(ZIndex::Global(1))
                                .background_color(BackgroundColor(CELL_BACKGROUND_COLOR))
                                .border_color(BorderColor(CELL_DARK_BORDER_COLOR))
                                .child(
                                    El::<TextBundle>::new()
                                    .align(Align::center())
                                    .text_signal(
                                        cell_data.index.signal()
                                        .map(|i|
                                            Text::from_section(
                                                ITEM_NAMES.get(&i).unwrap().to_string(),
                                                TextStyle { font_size: 50., ..default() }
                                            )
                                            .with_no_wrap()
                                        )
                                    )
                                )
                        }))
                    )
                })
        )
}

fn random_cell_data(rng: &mut impl Rng) -> CellData {
    CellData {
        index: Mutable::new(rng.gen_range(0..ITEM_NAMES.len())),
        count: Mutable::new(rng.gen_range(1..=64)),
    }
}

fn bern_cell_data_option(bern: f64) -> Mutable<Option<CellData>> {
    Mutable::new('block: {
        let distribution = Bernoulli::new(bern).unwrap();
        let mut rng = rand::thread_rng();
        if distribution.sample(&mut rng) {
            break 'block Some(random_cell_data(&mut rng));
        }
        None
    })
}

fn bern_cell(bern: f64, insertable: bool) -> impl Element {
    cell(bern_cell_data_option(bern), insertable)
}

fn grid<I: IntoIterator<Item = Mutable<Option<CellData>>>>(cell_data_options: I) -> impl Element
where
    <I as IntoIterator>::IntoIter: std::marker::Send + 'static,
{
    Grid::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .with_style(|mut style| {
            style.column_gap = Val::Px(CELL_GAP);
            style.row_gap = Val::Px(CELL_GAP);
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

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn dot() -> impl Element {
    El::<NodeBundle>::new()
        .width(Val::Px(CELL_BORDER_WIDTH * 2.))
        .height(Val::Px(CELL_BORDER_WIDTH * 2.))
        .background_color(BackgroundColor(CELL_BACKGROUND_COLOR))
}

fn dot_row(n: usize) -> impl Element {
    Row::<NodeBundle>::new().items((0..n).map(|_| dot()))
}

fn arrow() -> impl Element {
    Column::<NodeBundle>::new()
        .align_content(Align::center())
        .items((0..=6).map(|i| dot_row(2 * i + 1)))
        .items((0..6).map(|_| dot_row(3)))
}

fn side_column() -> impl Element {
    Column::<NodeBundle>::new()
        .with_style(|mut style| style.row_gap = Val::Px(CELL_GAP))
        .items((0..4).map(|_| bern_cell(0.5, true)))
}

fn inventory() -> impl Element {
    El::<NodeBundle>::new()
        .align(Align::center())
        .height(Val::Px(INVENTORY_SIZE))
        .width(Val::Px(INVENTORY_SIZE))
        .child(
            Column::<NodeBundle>::new()
            .height(Val::Percent(100.))
            .width(Val::Percent(100.))
                .with_style(|mut style| style.row_gap = Val::Px(CELL_GAP * 4.))
                .background_color(BackgroundColor(INVENTORY_BACKGROUND_COLOR))
                .align_content(Align::center())
                .item(
                    Row::<NodeBundle>::new()
                    .width(Val::Percent(100.))
                        .with_style(|mut style| style.column_gap = Val::Px(CELL_GAP))
                        .item(
                            Row::<NodeBundle>::new()
                                .align_content(Align::center())
                                .width(Val::Percent(60.))
                                .with_style(|mut style| {
                                    style.column_gap = Val::Px(CELL_GAP);
                                    style.padding = UiRect::horizontal(Val::Px(CELL_GAP * 3.));
                                })
                                .item(side_column())
                                .item(
                                    El::<NodeBundle>::new()
                                        .height(Val::Px(CELL_WIDTH * 4. + CELL_GAP * 3.))
                                        .width(Val::Percent(100.))
                                        .background_color(BackgroundColor(Color::BLACK)),
                                )
                                .item(side_column())
                        )
                        .item(
                            El::<NodeBundle>::new()
                            .width(Val::Percent(40.))
                            .height(Val::Percent(100.))
                                .align_content(Align::center())
                                .child({
                                    let inputs = MutableVec::new_with_values(
                                        (0..4).map(|_| bern_cell_data_option(0.2)).collect(),
                                    );
                                    let output: Mutable<Option<CellData>> = default();
                                    let outputter = spawn(clone!((inputs, output) async move {
                                        // TODO: explain every step of this signal
                                        inputs.signal_vec_cloned()
                                        .map_signal(|input|
                                            input.signal_cloned()
                                            // this says "retrigger" the outputter every time any of the input's
                                            // texture atlas index or count changes
                                            .map_some(|cell_data| map_ref! {
                                                let _ = cell_data.index.signal_ref(|_|()),
                                                let _ = cell_data.count.signal_ref(|_|()) => ()
                                            })
                                            .switch(signal::option)
                                        )
                                        .to_signal_map(|filleds| filleds.iter().map(Option::is_some).all(identity))
                                        .for_each_sync(move |all_filled| {
                                            output.set(all_filled.then(|| random_cell_data(&mut rand::thread_rng())));
                                        })
                                        .await;
                                    }));
                                    Column::<NodeBundle>::new()
                                        .update_raw_el(|raw_el| raw_el.hold_tasks([outputter]))
                                        .with_style(|mut style| {
                                            style.row_gap = Val::Px(CELL_GAP * 2.);
                                        })
                                        .item(
                                            // need to add another wrapping node here so the special output `Down`
                                            // handler doesn't overwrite the default `cell` `Down` handler
                                            El::<NodeBundle>::new()
                                            .child(cell(output.clone(), false).align(Align::center()))
                                            .update_raw_el(clone!((inputs) move |raw_el| {
                                                raw_el
                                                .component_signal::<On::<Pointer<Down>>, _>(
                                                    signal::and(DRAGGING_OPTION.signal_ref(Option::is_none), output.signal_ref(Option::is_some)).dedupe()
                                                    .map_true(move || {
                                                        On::<Pointer<Down>>::run(clone!((inputs) move || {
                                                            for input in inputs.lock_ref().iter() {
                                                                input.take();
                                                            }
                                                        }))
                                                    })
                                                )
                                            }))
                                        )
                                        .item(arrow())
                                        .item({
                                            let cell_data_options = inputs.lock_ref().iter().cloned().collect::<Vec<_>>();
                                            El::<NodeBundle>::new()
                                                .width(Val::Px(CELL_WIDTH * 2. + CELL_GAP))
                                                .child(grid(cell_data_options).align_content(Align::new().center_x()))
                                        })
                                }),
                        ),
                )
                .item(
                    El::<NodeBundle>::new()
                        .width(Val::Percent(100.))
                        .child(
                            grid((0..27).map(|_| bern_cell_data_option(0.5)))
                                .align_content(Align::new().center_x()),
                        ),
                )
                .item(
                    Row::<NodeBundle>::new()
                        .with_style(|mut style| {
                            style.column_gap = Val::Px(CELL_GAP);
                        })
                        .items((0..9).map(|_| bern_cell(0.5, true))),
                ),
        )
}

static DRAGGING_OPTION: Lazy<Mutable<Option<Mutable<Option<CellData>>>>> = Lazy::new(default);

static POINTER_POSITION: Lazy<Mutable<(f32, f32)>> = Lazy::new(default);

fn is_dragging() -> impl Signal<Item = bool> {
    DRAGGING_OPTION.signal_ref(Option::is_some)
}

fn ui_root() -> impl Element {
    Stack::<NodeBundle>::new()
        .cursor_disableable_signal(CursorIcon::Default, is_dragging())
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .update_raw_el(|raw_el| {
            raw_el
                .insert(On::<Pointer<Move>>::run(|move_: Listener<Pointer<Move>>| {
                    POINTER_POSITION.set(move_.pointer_location.position.into());
                }))
                .component_signal::<Pickable, _>(is_dragging().map_true(default))
        })
        .align_content(Align::center())
        .layer(inventory())
        .layer_signal(
            DRAGGING_OPTION
                .signal_cloned()
                .map_some(|cell_data_option| cell_data_option.signal_cloned())
                .switch(signal::option)
                .map(Option::flatten)
                .map_some(move |cell_data| {
                    icon(cell_data.index.signal(), cell_data.count.signal())
                        .update_raw_el(|raw_el| {
                            raw_el.defer_update(DeferredUpdaterAppendDirection::Front, |raw_el| {
                                raw_el.insert(Pickable {
                                    // required to allow cell hover to leak through a dragging icon
                                    should_block_lower: false,
                                    is_hoverable: true,
                                })
                            })
                        })
                        .cursor(CursorIcon::Grabbing)
                        .width(Val::Px(CELL_WIDTH))
                        .height(Val::Px(CELL_WIDTH))
                        .with_style(|mut style| {
                            style.position_type = PositionType::Absolute;
                            let pointer_position = POINTER_POSITION.get();
                            // TODO: this is actually *extremely* cringe, because the `.on_signal_with_style`
                            // will(might?) not tick before the first frame the icon is
                            // rendered, the icon will flash from the left middle of the screen (default absolute
                            // position?) to the pointer position, this means that the
                            // position must first be set statically here *and* in reaction
                            // to the pointer position below; workaround could be to wait
                            // for a tick before making the the element visible, but
                            // *ideally* we would force all signals to tick before the first frame, but not
                            // sure if that's possible
                            set_dragging_position(style, pointer_position);
                        })
                        .z_index(ZIndex::Global(1))
                        .on_signal_with_style(POINTER_POSITION.signal(), set_dragging_position)
                }),
        )
}

fn set_dragging_position(mut style: Mut<Style>, pointer_position: (f32, f32)) {
    style.left = Val::Px(pointer_position.0 - CELL_WIDTH / 2.);
    style.top = Val::Px(pointer_position.1 - CELL_WIDTH / 2.);
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum AssetState {
    #[default]
    Loading,
    Loaded,
}
