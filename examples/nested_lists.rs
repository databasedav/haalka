//! Nested dynamic lists, arbitrarily deeply nested retained reactivity, spurred by <https://discord.com/channels/691052431525675048/885021580353237032/1356769984474517617>

mod utils;
use bevy_color::palettes::css::DARK_GRAY;
use utils::*;

use bevy::prelude::*;
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

static MASTER: LazyLock<Lists> = LazyLock::new(default);

fn lists_element(lists: Lists) -> Column<Node> {
    let Lists { lists } = lists;
    Column::<Node>::new().item(
        Row::<Node>::new()
            .with_node(|mut node| node.column_gap = Val::Px(10.))
            .item(
                El::<Node>::new()
                    .align(Align::new().top())
                    .with_node(|mut node| node.width = Val::Px(80.))
                    .with_node(|mut node| node.height = Val::Px(40.))
                    .background_color(BackgroundColor(random_color())),
            )
            .item(
                Column::<Node>::new()
                    .with_node(|mut node| node.row_gap = Val::Px(10.))
                    .items_signal_vec(lists.signal_vec_cloned().map(lists_element))
                    .item(
                        El::<Node>::new()
                            .with_node(|mut node| node.width = Val::Px(30.))
                            .with_node(|mut node| node.height = Val::Px(30.))
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
        .with_node(|mut node| node.width = Val::Percent(100.))
        .with_node(|mut node| node.height = Val::Percent(100.))
        .cursor(CursorIcon::default())
        .align_content(Align::new().top().left())
        .child(
            lists_element(MASTER.clone())
                .with_node(|mut node| {
                    node.left = Val::Px(20.);
                    node.top = Val::Px(20.);
                })
                .with_node(|mut node| node.height = Val::Percent(100.))
                .mutable_viewport(haalka::prelude::Axis::Vertical)
                .on_scroll_with_system(BasicScrollHandler::new().pixels(20.).into_system()),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
