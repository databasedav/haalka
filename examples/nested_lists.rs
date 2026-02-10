//! Nested dynamic lists, arbitrarily deeply nested retained reactivity, spurred by <https://discord.com/channels/691052431525675048/885021580353237032/1356769984474517617>

mod utils;
use utils::*;

use bevy::{color::palettes::css::DARK_GRAY, prelude::*, ui::Overflow};
use bevy_rand::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((examples_plugin, EntropyPlugin::<WyRand>::default()))
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    let master = Lists::new(world);
                    ui_root(master).spawn(world);
                },
                camera,
            ),
        )
        .run();
}

#[derive(Clone)]
struct Lists {
    lists: MutableVec<Lists>,
}

impl Lists {
    fn new(world: &mut World) -> Self {
        Self {
            lists: MutableVec::from(world),
        }
    }
}

/// Component to store the current index of a list item
#[derive(Component, Clone, Copy, Deref)]
struct Index(usize);

fn list_item_box(
    index_signal: impl Signal<Item = Option<usize>>,
    parent_lists_vec_option: Option<MutableVec<Lists>>,
) -> impl Element {
    let has_parent = parent_lists_vec_option.is_some();
    let el = El::<Node>::new()
        .align(Align::new().top())
        .with_node(|mut node| {
            node.width = Val::Px(80.);
            node.height = Val::Px(40.);
        })
        .with_builder(|builder| {
            builder.on_spawn_with_system(
                |In(entity): In<Entity>,
                 mut rng: Single<&mut WyRand, With<GlobalRng>>,
                 mut backgrounds: Query<&mut BackgroundColor>| {
                    if let Ok(mut bg) = backgrounds.get_mut(entity) {
                        *bg = BackgroundColor(random_color(rng.as_mut()));
                    }
                },
            )
        })
        .cursor(if has_parent {
            CursorIcon::System(SystemCursorIcon::Pointer)
        } else {
            CursorIcon::default()
        });

    if let Some(parent_vec) = parent_lists_vec_option {
        el.insert(Pickable::default())
            .component_signal(index_signal.map_some_in(Index))
            .child(
                El::<Text>::new()
                    .align(Align::center())
                    .text_font(TextFont::from_font_size(30.))
                    .text_color(TextColor(Color::WHITE))
                    .text(Text::from("-")),
            )
            .on_click(
                move |In((entity, _)): In<(Entity, Pointer<Click>)>,
                      indices: Query<&Index>,
                      mut vec_datas: Query<&mut MutableVecData<Lists>>| {
                    let index = **indices.get(entity).unwrap();
                    parent_vec.write(&mut vec_datas).remove(index);
                },
            )
    } else {
        el
    }
}

fn add_button(child_lists_vec: MutableVec<Lists>) -> El<Node> {
    El::<Node>::new()
        .insert(Pickable::default())
        .with_node(|mut node| {
            node.width = Val::Px(30.);
            node.height = Val::Px(30.);
        })
        .background_color(BackgroundColor(DARK_GRAY.into()))
        .align_content(Align::center())
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .on_click(move |In(_), world: &mut World| {
            let new_lists = Lists::new(world);
            child_lists_vec.write(world).push(new_lists);
        })
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(30.))
                .text_color(TextColor(Color::WHITE))
                .text(Text::from("+")),
        )
}

fn nested_lists(child_lists_vec: MutableVec<Lists>) -> Column<Node> {
    Column::<Node>::new()
        .with_node(|mut node| node.row_gap = Val::Px(10.))
        .items_signal_vec(child_lists_vec.signal_vec().enumerate().map(
            clone!((child_lists_vec) move |In((i, lists)): In<(BoxedSignal<Option<usize>>, Lists)>| {
                lists_element(i, lists, Some(child_lists_vec.clone()))
            }),
        ))
        .item(add_button(child_lists_vec))
}

fn lists_element(
    index_signal: impl Signal<Item = Option<usize>>,
    child_lists: Lists,
    parent_lists_vec_option: Option<MutableVec<Lists>>,
) -> Column<Node> {
    let Lists { lists: child_lists_vec } = child_lists;
    Column::<Node>::new().item(
        Row::<Node>::new()
            .with_node(|mut node| node.column_gap = Val::Px(10.))
            .item(list_item_box(index_signal, parent_lists_vec_option))
            .item(nested_lists(child_lists_vec)),
    )
}

fn ui_root(master: Lists) -> impl Element {
    El::<Node>::new()
        .with_node(|mut node| {
            node.width = Val::Percent(100.);
            node.height = Val::Percent(100.);
        })
        .insert(Pickable::default())
        .cursor(CursorIcon::default())
        .align_content(Align::new().top().left())
        .child(
            lists_element(signal::once(None), master, None)
                .insert(Pickable::default())
                .with_node(|mut node| {
                    node.height = Val::Percent(100.);
                    node.left = Val::Px(20.);
                    node.top = Val::Px(20.);
                })
                .mutable_viewport(Overflow::scroll_y())
                .on_scroll(BasicScrollHandler::new().pixels(20.).into_system()),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
