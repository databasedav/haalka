//! nested dynamic lists

mod utils;
use rand::seq::SliceRandom;
use utils::*;

use bevy::{color::palettes::css::*, prelude::*};
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins(examples_plugin)
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
                },
                camera,
            ),
        )
        .run();
}

#[derive(Clone, Default)]
struct Lists {
    lists: MutableVec<Lists>,
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

static MASTER: Lazy<Lists> = Lazy::new(default);

fn random_color() -> Color {
    let mut rng = rand::thread_rng();
    COLORS.choose(&mut rng).copied().unwrap()
}

fn lists_element(lists: Lists) -> Column<Node> {
    let Lists { lists } = lists;
    Column::<Node>::new().item(
        Row::<Node>::new()
            .with_node(|mut node| node.column_gap = Val::Px(10.))
            .item(
                El::<Node>::new()
                    .align(Align::new().top())
                    .width(Val::Px(80.))
                    .height(Val::Px(40.))
                    .background_color(BackgroundColor(random_color())),
            )
            .item(
                Column::<Node>::new()
                    .with_node(|mut node| node.row_gap = Val::Px(10.))
                    .items_signal_vec(lists.signal_vec_cloned().map(lists_element))
                    .item(
                        El::<Node>::new()
                            .width(Val::Px(30.))
                            .height(Val::Px(30.))
                            .background_color(BackgroundColor(DARK_GRAY.into()))
                            .align_content(Align::center())
                            .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                            .child(
                                El::<Text>::new()
                                    .text_font(TextFont::from_font_size(30.))
                                    .text_color(TextColor(Color::WHITE))
                                    .text(Text::from("+"))
                                    .on_click(move || {
                                        lists.lock_mut().push_cloned(default());
                                    }),
                            ),
                    ),
            ),
    )
}

fn ui_root() -> impl Element {
    El::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .cursor(CursorIcon::System(SystemCursorIcon::Default))
        .align_content(Align::new().top().left())
        .child(
            lists_element(MASTER.clone())
                .with_node(|mut node| {
                    node.left = Val::Px(20.);
                    node.top = Val::Px(20.);
                })
                .height(Val::Percent(100.))
                .mutable_viewport(Overflow::clip_y(), None)
                .on_scroll_with_system(BasicScrollHandler::new().pixels(20.).into_system()),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
