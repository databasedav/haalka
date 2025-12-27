//! Nested dynamic lists, arbitrarily deeply nested retained reactivity, spurred by <https://discord.com/channels/691052431525675048/885021580353237032/1356769984474517617>

mod utils;
use utils::*;

use bevy::{color::palettes::css::DARK_GRAY, prelude::*, ui::Overflow};
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins(examples_plugin)
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
            lists: MutableVecBuilder::from(Vec::<Lists>::new()).spawn(world),
        }
    }
}

/// Component to store the current index of a list item
#[derive(Component, Clone, Copy)]
struct ListItemIndex(usize);

/// Component to store the parent MutableVec reference for removal
#[derive(Component, Clone)]
struct ParentListsVec(MutableVec<Lists>);

fn lists_element(
    index_signal: signal::Source<Option<usize>>,
    child_lists: Lists,
    parent_lists_vec_option: Option<MutableVec<Lists>>,
) -> Column<Node> {
    let Lists { lists: child_lists_vec } = child_lists;
    Column::<Node>::new().item(
        Row::<Node>::new()
            .with_node(|mut node| node.column_gap = Val::Px(10.))
            .item({
                let has_parent = parent_lists_vec_option.is_some();
                let el = El::<Node>::new()
                    .align(Align::new().top())
                    .with_node(|mut node| {
                        node.width = Val::Px(80.);
                        node.height = Val::Px(40.);
                    })
                    .background_color(BackgroundColor(random_color()))
                    .cursor(if has_parent {
                        CursorIcon::System(SystemCursorIcon::Pointer)
                    } else {
                        CursorIcon::default()
                    });

                if let Some(parent_vec) = parent_lists_vec_option {
                    el.insert((Pickable::default(), ParentListsVec(parent_vec)))
                        // Store the index as a component that updates reactively
                        .with_builder(|builder| builder.component_signal(index_signal.map_some_in(ListItemIndex)))
                        .child(
                            El::<Text>::new()
                                .align(Align::center())
                                .text_font(TextFont::from_font_size(30.))
                                .text_color(TextColor(Color::WHITE))
                                .text(Text::from("-")),
                        )
                        .on_click(
                            |In((entity, _)): In<(Entity, Pointer<Click>)>,
                             indices: Query<&ListItemIndex>,
                             parent_vecs: Query<&ParentListsVec>,
                             mut vec_datas: Query<&mut MutableVecData<Lists>>| {
                                let index = indices.get(entity).ok().map(|i| i.0);
                                let parent_vec = parent_vecs.get(entity).ok().map(|p| p.0.clone());
                                if let (Some(index), Some(parent_vec)) = (index, parent_vec) {
                                    parent_vec.write(&mut vec_datas).remove(index);
                                }
                            },
                        )
                } else {
                    el
                }
            })
            .item(
                Column::<Node>::new()
                    .with_node(|mut node| node.row_gap = Val::Px(10.))
                    .items_signal_vec(child_lists_vec.signal_vec().enumerate().map(
                        clone!((child_lists_vec) move |In((i, lists)): In<(signal::Source<Option<usize>>, Lists)>| {
                            lists_element(i, lists, Some(child_lists_vec.clone()))
                        }),
                    ))
                    .item({
                        let child_lists_for_add = child_lists_vec.clone();
                        El::<Node>::new()
                            .insert(Pickable::default())
                            .with_node(|mut node| {
                                node.width = Val::Px(30.);
                                node.height = Val::Px(30.);
                            })
                            .background_color(BackgroundColor(DARK_GRAY.into()))
                            .align_content(Align::center())
                            .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                            .on_click(move |In(_): In<_>, world: &mut World| {
                                let new_lists = Lists::new(world);
                                child_lists_for_add.write(world).push(new_lists);
                            })
                            .child(
                                El::<Text>::new()
                                    .text_font(TextFont::from_font_size(30.))
                                    .text_color(TextColor(Color::WHITE))
                                    .text(Text::from("+")),
                            )
                    }),
            ),
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
            lists_element(SignalBuilder::always(None), master, None)
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
