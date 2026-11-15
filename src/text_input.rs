//! Reactive text input widget and adjacent utilities built on Bevy's native [`EditableText`]
//! and integrated with [`Signal`]s.

use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, system::*};
use bevy_input_focus::{FocusCause, InputFocus};
use bevy_picking::prelude::*;
use bevy_platform::sync::{Arc, OnceLock};
use bevy_text::{EditableText, LineHeight, TextColor, TextCursorStyle, TextEdit, TextFont, TextLayout};
use bevy_ui::prelude::*;
use bevy_ui_widgets::EditableTextInputPlugin;
use jonmo::signal::{Signal, SignalExt};
use paste::paste;

use crate::{element::BuilderPassThrough, impl_haalka_methods};

use super::{
    el::El,
    element::{BuilderWrapper, ElementWrapper, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{Cursorable, PointerEventAware},
    utils::{clone, observe, register_system, remove_system_holder_on_despawn},
    viewport_mutable::ViewportMutable,
};

/// Reactive text input widget built on Bevy's native [`EditableText`] and integrated with
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

impl BuilderPassThrough for TextInput {}
impl GlobalEventAware for TextInput {}
impl Nameable for TextInput {}
impl PointerEventAware for TextInput {}
impl MouseWheelScrollable for TextInput {}
impl UiRootable for TextInput {}
impl ViewportMutable for TextInput {}
impl Cursorable for TextInput {}

impl TextInput {
    #[allow(missing_docs, clippy::new_without_default)]
    pub fn new() -> Self {
        let el = El::<Node>::new().with_builder(|builder| {
            builder.insert((
                EditableText::default(),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                Pickable::default(),
            ))
        });
        Self { el }
    }

    /// Run a function with this input's native [`EditableText`] component.
    pub fn with_buffer(self, f: impl FnOnce(Mut<EditableText>) + Send + Sync + 'static) -> Self {
        self.with_builder(|builder| {
            builder.on_spawn(move |world, entity| {
                let mut system_state: SystemState<Query<&mut EditableText>> = SystemState::new(world);
                if let Ok(mut editable_texts) = system_state.get_mut(world)
                    && let Ok(editable_text) = editable_texts.get_mut(entity)
                {
                    f(editable_text);
                }
            })
        })
    }

    /// Reactively run a function with this input's native [`EditableText`] component and the
    /// output of the [`Signal`].
    pub fn on_signal_with_buffer<T: Send + Sync + 'static>(
        self,
        signal: impl Signal<Item = T> + 'static,
        mut f: impl FnMut(Mut<EditableText>, T) + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(move |builder| {
            builder.on_signal(
                signal,
                move |In((entity, value)): In<(Entity, T)>, mut editable_texts: Query<&mut EditableText>| {
                    if let Ok(editable_text) = editable_texts.get_mut(entity) {
                        f(editable_text, value);
                    }
                },
            )
        })
    }

    /// Set the text of this input.
    pub fn text(self, text_option: impl Into<Option<String>>) -> Self {
        let text = text_option.into().unwrap_or_default();
        self.with_buffer(move |mut editable_text| set_text(&mut editable_text, text))
    }

    /// Reactively set the text of this input. If the signal outputs [`None`], the text is set to
    /// an empty string.
    pub fn text_signal<T: Into<Option<String>> + Send + Sync + 'static, S: Signal<Item = T> + Send + 'static>(
        mut self,
        text_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_option_signal) = text_option_signal_option.into() {
            self = self.on_signal_with_buffer(
                text_option_signal.map_in(|text_option: T| text_option.into().unwrap_or_default()),
                |mut editable_text, text| set_text(&mut editable_text, text),
            );
        }
        self
    }

    /// When this input's focused state changes, run a system which takes [`In`](`System::In`)
    /// this input's [`Entity`] and its current focused state.
    pub fn on_focused_change<Marker>(
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
                        commands.run_system_with(system, (entity, event.event().focused));
                    });
                }))
                .apply(remove_system_holder_on_despawn(system_holder.clone()))
        })
    }

    /// Set the focused state of this input.
    pub fn focus_option(mut self, focus_option: impl Into<Option<bool>>) -> Self {
        if focus_option.into().unwrap_or(false) {
            self = self.with_builder(|builder| {
                builder.on_spawn_with_system(|In(entity), mut input_focus: ResMut<InputFocus>| {
                    input_focus.set(entity, FocusCause::Navigated);
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
                    |In((entity, focus)), mut input_focus: ResMut<InputFocus>| {
                        if focus {
                            input_focus.set(entity, FocusCause::Navigated);
                        } else if input_focus.get() == Some(entity) {
                            input_focus.clear();
                        }
                    },
                )
            });
        }
        self
    }

    /// When the string in this input changes, run a `handler` [`System`] which takes
    /// [`In`](System::In) the [`Entity`] of this input and the new [`String`].
    pub fn on_change<Marker>(
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
                        commands.run_system_with(system, (change.event().entity, change.event().text.clone()));
                    });
                }))
                .with_entity(|mut entity| {
                    entity.insert_if_new(ListenToChanges::default());
                })
                .apply(remove_system_holder_on_despawn(system_holder))
        })
    }
}

fn set_text(editable_text: &mut EditableText, text: String) {
    if editable_text.value().to_string() != text {
        editable_text.queue_edit(TextEdit::SelectAll);
        editable_text.queue_edit(TextEdit::Insert(text.into()));
    }
}

#[derive(Component, Default)]
struct ListenToChanges(String);

#[derive(EntityEvent)]
struct TextInputChange {
    entity: Entity,
    text: String,
}

fn on_change(
    mut editable_texts: Query<(Entity, &EditableText, &mut ListenToChanges), Changed<EditableText>>,
    mut commands: Commands,
) {
    for (entity, editable_text, mut last_text) in &mut editable_texts {
        let text = editable_text.value().to_string();
        if last_text.0 != text {
            last_text.0.clone_from(&text);
            commands.trigger(TextInputChange { entity, text });
        }
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
    input_focus: Res<InputFocus>,
    mut text_inputs: Query<(Entity, &mut Focusable)>,
    mut commands: Commands,
) {
    for (entity, mut focusable) in &mut text_inputs {
        let is_focused = input_focus.get() == Some(entity);
        if focusable.is_focused != is_focused {
            focusable.is_focused = is_focused;
            commands.trigger(FocusedChange {
                entity,
                focused: is_focused,
            });
        }
    }
}

impl_haalka_methods! {
    TextInput {
        node: Node,
        editable_text: EditableText,
        text_font: TextFont,
        text_layout: TextLayout,
        line_height: LineHeight,
        text_color: TextColor,
        text_cursor_style: TextCursorStyle,
    }
}

pub(super) fn plugin(app: &mut App) {
    if !app.is_plugin_added::<EditableTextInputPlugin>() {
        app.add_plugins(EditableTextInputPlugin);
    }
    app.add_systems(
        Update,
        (
            on_change.run_if(any_with_component::<ListenToChanges>),
            on_focus_changed.run_if(resource_changed_or_removed::<InputFocus>),
        )
            .run_if(any_with_component::<EditableText>),
    );
}
