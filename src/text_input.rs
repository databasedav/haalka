//! Reactive text input widget and adjacent utilities, a thin wrapper around [`bevy_ui_text_input`]
//! integrated with [`Signal`]s.

use std::{
    ops::Not,
    sync::{Arc, OnceLock},
};

use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, system::*};
use bevy_input_focus::InputFocus;
use bevy_picking::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_ui::prelude::*;
use bevy_utils::prelude::*;
use cosmic_text::{Edit, Selection};

use crate::impl_haalka_methods;

use super::{
    el::El,
    element::{BuilderWrapper, ElementWrapper, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, PointerEventAware},
    utils::{clone, observe, register_system, remove_system_holder_on_remove},
    viewport_mutable::ViewportMutable,
};
use apply::Apply;
use bevy_ui_text_input::{actions::TextInputAction, text_input_pipeline::TextInputPipeline, *};
use jonmo::signal::{Signal, SignalExt};
use paste::paste;

/// Reactive text input widget, a thin wrapper around [`bevy_ui_text_input`] integrated with
/// [`Signal`]s.
#[derive(Default)]
pub struct TextInput {
    el: El<Node>,
}

impl ElementWrapper for TextInput {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl GlobalEventAware for TextInput {}
impl Nameable for TextInput {}
impl PointerEventAware for TextInput {}
impl MouseWheelScrollable for TextInput {}
impl UiRootable for TextInput {}
impl ViewportMutable for TextInput {}
impl CursorOnHoverable for TextInput {}

// TODO: allow managing multiple spans reactively
impl TextInput {
    #[allow(missing_docs, clippy::new_without_default)]
    pub fn new() -> Self {
        let el = El::<Node>::new().with_builder(|builder| {
            builder.insert((
                TextInputNode {
                    clear_on_submit: false,
                    ..default()
                },
                Pickable::default(),
                LastSignalText::default(),
            ))
        });
        Self { el }
    }

    /// Run a function with this input's [`TextInputBuffer`] with access to
    /// [`ResMut<TextInputPipeline>`].
    pub fn with_buffer(
        self,
        f: impl FnOnce(Mut<TextInputBuffer>, ResMut<TextInputPipeline>) + Send + Sync + 'static,
    ) -> Self {
        // .on_spawn_with_system doesn't work because it requires FnMut
        self.with_builder(|builder| {
            builder.on_spawn(move |world, entity| {
                // TODO: is this stuff repeated for every call ?
                #[allow(clippy::type_complexity)]
                let mut system_state: SystemState<(
                    Query<&mut TextInputBuffer>,
                    ResMut<TextInputPipeline>,
                )> = SystemState::new(world);
                let (mut buffers, text_input_pipeline) = system_state.get_mut(world);
                if let Ok(buffer) = buffers.get_mut(entity) {
                    f(buffer, text_input_pipeline)
                }
            })
        })
    }

    /// Reactively run a function with this input's [`TextInputBuffer`] and the output of the
    /// [`Signal`] with access to [`ResMut<TextInputPipeline>`].
    pub fn on_signal_with_buffer<T: Clone + Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Mut<TextInputBuffer>, ResMut<TextInputPipeline>, T) + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(move |builder| {
            builder.on_signal(
                signal,
                move |In((entity, value)): In<(Entity, T)>,
                      mut buffers: Query<&mut TextInputBuffer>,
                      text_input_pipeline: ResMut<TextInputPipeline>| {
                    if let Ok(buffer) = buffers.get_mut(entity) {
                        f(buffer, text_input_pipeline, value)
                    };
                },
            )
        })
    }

    /// Set the text of this input.
    pub fn text(self, text_option: impl Into<Option<String>>) -> Self {
        let text = text_option.into().unwrap_or_default();
        self.with_text_input_queue(move |mut text_input_queue| {
            queue_set_text_actions(&mut text_input_queue, text);
        })
    }

    /// Reactively set the text of this input. If the signal outputs [`None`] the text is set to an
    /// empty string.
    pub fn text_signal<
        T: Into<Option<String>> + Clone + Send + Sync + 'static,
        S: Signal<Item = T> + Send + 'static,
    >(
        mut self,
        text_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_option_signal) = text_option_signal_option.into() {
            self = self.with_builder(|builder| {
                builder.on_signal(
                    text_option_signal.map_in(|text_option: T| text_option.into().unwrap_or_default()),
                    |In((entity, text)): In<(Entity, String)>,
                     mut last_text_query: Query<&mut LastSignalText>,
                     mut text_input_queues: Query<&mut TextInputQueue>| {
                        if let Ok(mut last_text) = last_text_query.get_mut(entity) {
                            // only queue an update if the incoming signal value is different
                            // from the last value we set from a signal. This prevents redundant updates.
                            if last_text.0 != text {
                                last_text.0 = text.clone();
                                if let Ok(mut queue) = text_input_queues.get_mut(entity) {
                                    queue_set_text_actions(&mut queue, text);
                                }
                            }
                        }
                    },
                )
            });
        }
        self
    }

    /// When this input's focused state changes, run a system which takes [`In`](`System::In`)
    /// this input's [`Entity`] and its current focused state.
    pub fn on_focused_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, bool)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .with_entity(|mut entity| {
                    entity.insert(Focusable { is_focused: false });
                })
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |event: On<FocusedChange>, mut commands: Commands| {
                        commands.run_system_with(system, (entity, event.event().focused))
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        })
    }

    /// When this input's focused state changes, run a function with its current focused state.
    pub fn on_focused_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_focused_change_with_system(move |In((_, is_focused))| handler(is_focused))
    }

    /// Set the focused state of this input.
    pub fn focus_option(mut self, focus_option: impl Into<Option<bool>>) -> Self {
        if Into::<Option<bool>>::into(focus_option).unwrap_or(false) {
            self = self.with_builder(|builder| {
                builder.on_spawn_with_system(|In(entity), mut commands: Commands| {
                    commands.insert_resource(InputFocus(Some(entity)));
                })
            });
        }
        self
    }

    /// Focus this input.
    pub fn focus(self) -> Self {
        self.focus_option(true)
    }

    /// Reactively focus this input.
    pub fn focus_signal<S: Signal<Item = bool> + Send + 'static>(
        mut self,
        focus_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(focus_signal) = focus_signal_option.into() {
            self = self.with_builder(|builder| {
                builder.on_signal(
                    focus_signal,
                    |In((entity, focus)), mut focused_option: ResMut<InputFocus>| {
                        if focus {
                            focused_option.0 = Some(entity);
                        } else if let Some(focused) = focused_option.0
                            && focused == entity
                        {
                            focused_option.0 = None;
                        }
                    },
                )
            })
        }
        self
    }

    /// When the string in this input changes, run a `handler` [`System`] which takes
    /// [`In`](System::In) the [`Entity`] of this input's [`Entity`] and the new [`String`].
    pub fn on_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, String)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |change: On<TextInputChange>, mut commands: Commands| {
                        let TextInputChange { entity, text } = change.event();
                        commands.run_system_with(system, (*entity, text.clone()));
                    });
                }))
                .with_entity(|mut entity| {
                    entity.insert_if_new((ListenToChanges, TextInputContents::default()));
                })
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When the text of this input changes, run a function with the new text.
    pub fn on_change(self, mut handler: impl FnMut(String) + Send + Sync + 'static) -> Self {
        self.on_change_with_system(move |In((_, text))| handler(text))
    }
}

/// A component to store the last text value that was successfully applied by
/// [`TextInput::text_signal`]. This is used to prevent echo updates from
/// [`TextInput::on_change_sync`] when a two-way binding is active.
#[derive(Component, Default)]
struct LastSignalText(String);

fn queue_set_text_actions(text_input_queue: &mut TextInputQueue, text: String) {
    for action in [
        TextInputAction::Edit(actions::TextInputEdit::SelectAll),
        TextInputAction::Edit(actions::TextInputEdit::Paste(text)),
    ] {
        text_input_queue.add(action);
    }
}

#[derive(Component)]
struct ListenToChanges;

#[derive(EntityEvent)]
struct TextInputChange {
    entity: Entity,
    text: String,
}

#[allow(clippy::type_complexity)]
fn on_change(
    contents: Query<(Entity, &TextInputContents), (Changed<TextInputContents>, With<ListenToChanges>)>,
    mut commands: Commands,
) {
    for (entity, contents) in contents.iter() {
        commands.trigger(TextInputChange {
            entity,
            text: contents.get().to_string(),
        });
    }
}

#[derive(EntityEvent)]
struct FocusedChange {
    entity: Entity,
    focused: bool,
}

#[derive(Component)]
struct Focusable {
    is_focused: bool,
}

fn on_focus_changed(
    focused_option: Res<InputFocus>,
    mut text_inputs: Query<(Entity, &mut Focusable)>,
    mut commands: Commands,
) {
    for (entity, mut focusable) in text_inputs.iter_mut() {
        if Some(entity) == focused_option.0 {
            // TODO: remove condition when https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
            if focusable.is_focused.not() {
                focusable.is_focused = true;
                commands.trigger(FocusedChange { entity, focused: true });
            }
        } else if focusable.is_focused {
            focusable.is_focused = false;
            commands.trigger(FocusedChange { entity, focused: false });
        }
    }
}

impl_haalka_methods! {
    TextInput {
        node: Node,
        text_input_node: TextInputNode,
        #[skip_signal]
        text_input_buffer: TextInputBuffer,
        text_font: TextFont,
        text_input_layout_info: TextInputLayoutInfo,
        text_input_style: TextInputStyle,
        text_color: TextColor,
        text_input_prompt: TextInputPrompt,
        #[skip_signal]
        text_input_queue: TextInputQueue,
    }
}

/// Marker [`Resource`] to prevent clearing selection on focus change.
#[derive(Resource, Default)]
pub struct ClearSelectionOnFocusChangeDisabled;

fn clear_selection_on_focus_change(
    input_focus: Res<InputFocus>,
    mut text_input_pipeline: ResMut<TextInputPipeline>,
    mut buffers: Query<&mut TextInputBuffer>,
    mut previous_input_focus: Local<Option<Entity>>,
) {
    if *previous_input_focus != input_focus.0 {
        if let Some(entity) = *previous_input_focus
            && let Ok(mut buffer) = buffers.get_mut(entity)
        {
            buffer
                .editor
                .borrow_with(&mut text_input_pipeline.font_system)
                .set_selection(Selection::None);
        }
        *previous_input_focus = input_focus.0;
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(TextInputPlugin).add_systems(
        Update,
        (
            on_change.run_if(any_with_component::<ListenToChanges>),
            on_focus_changed.run_if(resource_changed_or_removed::<InputFocus>),
            clear_selection_on_focus_change.run_if(not(resource_exists::<ClearSelectionOnFocusChangeDisabled>)),
        )
            .run_if(any_with_component::<TextInputNode>),
    );
}
