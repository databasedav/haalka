//! Reactive text input widget and adjacent utilities, a thin wrapper around [`bevy_cosmic_edit`] integrated with [`Signal`]s.

use std::{ops::{Deref, Not}, pin::Pin, sync::{Arc, OnceLock}};

use bevy_input_focus::InputFocus;
use bevy_ecs::system::*;
use bevy_ecs::prelude::*;
use bevy_ui::{prelude::*, widget::NodeImageMode};
use bevy_color::prelude::*;
use bevy_utils::prelude::*;
use bevy_app::prelude::*;
use bevy_picking::prelude::*;
use bevy_text::{cosmic_text, TextColor, TextFont, DEFAULT_FONT_DATA};

use crate::impl_haalka_methods;

use super::{
    el::El, element::{ElementWrapper, Nameable, UiRootable}, pointer_event_aware::{PointerEventAware, CursorOnHoverable}, raw::{RawElWrapper, register_system}, mouse_wheel_scrollable::MouseWheelScrollable,
    sizeable::Sizeable, utils::clone, viewport_mutable::ViewportMutable, global_event_aware::GlobalEventAware,
    raw::{observe, utils::remove_system_holder_on_remove}
};
use apply::Apply;
use bevy_ui_text_input::{text_input_pipeline::TextInputPipeline, *};
use cosmic_text::FontSystem;
use futures_signals::signal::{always, BoxSignal, Mutable, Signal, SignalExt};
use haalka_futures_signals_ext::SignalExtBool;
use paste::paste;

/// Reactive text input widget, a thin wrapper around [`bevy_cosmic_edit`] integrated with [`Signal`]s.
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
impl Sizeable for TextInput {}
impl UiRootable for TextInput {}
impl ViewportMutable for TextInput {}
impl CursorOnHoverable for TextInput {}

/// Marker [`Component`] for [`TextInput`] to prevent focusing on [`Pointer<Pressed>`] events. Useful when input focus is more conditional.
#[derive(Component)]
pub struct TextInputFocusOnDownDisabled;

// TODO: allow managing multiple spans reactively
impl TextInput {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        let el = El::<Node>::new().update_raw_el(|raw_el| {
            raw_el
                .insert((
                    TextInputNode {
                        clear_on_submit: false,
                        ..default()
                    },
                    Pickable::default()
                ))
                // .on_event_with_system::<Pointer<Pressed>, _>(
                //     move |In((entity, _)): In<(_, Pointer<Pressed>)>,
                //             mut focusable_query: Query<(Entity, &mut Focusable), Without<TextInputFocusOnDownDisabled>>,
                //             mut commands: Commands| {
                //         // TODO: remove this focusable trigger and uncomment .insert_resource below when https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
                //         // otherwise cursor position is not instantly correct on `Down`
                //         if let Ok((entity, mut focusable)) = focusable_query.get_mut(entity) {
                //             focusable.is_focused = true;
                //             commands.trigger_targets(FocusedChange(true), entity);
                //         }
                //         // commands.insert_resource(CosmicFocusedWidget(cosmic_edit_holder.get()));
                //     },
                // )
        });
        Self { el }
    }

    /// Run a function with this input's [`CosmicEditBuffer`] with access to [`ResMut<CosmicFontSystem>`] and [`DefaultAttrs`].
    pub fn with_buffer(
        self,
        f: impl FnOnce(Mut<TextInputBuffer>, ResMut<TextInputPipeline>) + Send + 'static,
    ) -> Self {
        // .on_spawn_with_system doesn't work because it requires FnMut
        self.update_raw_el(|raw_el| raw_el.on_spawn(move |world, entity| {
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
        }))
    }

    /// Reactively run a function with this input's [`CosmicEditBuffer`] and the output of the [`Signal`] with access to [`ResMut<CosmicFontSystem>`] and [`DefaultAttrs`].
    pub fn on_signal_with_buffer<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Mut<TextInputBuffer>, ResMut<TextInputPipeline>, T) + Send + Sync + 'static,
    ) -> Self {
        self.update_raw_el(move |raw_el| {
            raw_el.on_signal_with_system(
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
        self.with_buffer(move |mut buffer, _| {
            buffer.set_text(text);
        })
    }

    /// Reactively set the text of this input. If the signal outputs [`None`] the text is set to an empty string.
    pub fn text_signal<S: Signal<Item = impl Into<Option<String>>> + Send + 'static>(
        mut self,
        text_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_option_signal) = text_option_signal_option.into() {
            self = self.on_signal_with_buffer(
                text_option_signal.map(|text_option| text_option.into()),
                |mut buffer, _, text_option| {
                    buffer.set_text(text_option.unwrap_or_default());
                },
            );
        }
        self
    }

    /// When this input's focused state changes, run a system which takes [`In`](`System::In`)
    /// this input's [`Entity`] and its current focused state.
    pub fn on_focused_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, bool,)>, (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Arc::new(OnceLock::new());
            raw_el
            .with_entity(|mut entity| { entity.insert(Focusable { is_focused: false }); })
            .on_spawn(clone!((system_holder) move |world, entity| {
                let system = register_system(world, handler);
                let _ = system_holder.set(system);
                observe(world, entity, move |event: Trigger<FocusedChange>, mut commands: Commands| {
                    commands.run_system_with(system, (entity, event.event().0))
                });
            }))
            .apply(remove_system_holder_on_remove(system_holder.clone()))
        })
    }

    /// When this input's focused state changes, run a function with its current focused state.
    pub fn on_focused_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_focused_change_with_system(move |In((_, is_focused))| handler(is_focused))
    }

    /// Sync a [`Mutable`] with this input's focused state.
    pub fn focused_sync(self, focused: Mutable<bool>) -> Self {
        self.on_focused_change(move |is_focused| focused.set_neq(is_focused))
    }

    /// Set the focused state of this input.
    pub fn focus_option(mut self, focus_option: impl Into<Option<bool>>) -> Self {
        if Into::<Option<bool>>::into(focus_option).unwrap_or(false) {
            self = self.update_raw_el(|raw_el| raw_el.on_spawn_with_system(|In(entity), mut commands: Commands| {
                commands.insert_resource(InputFocus(Some(entity)));
            }));
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
            self = self.update_raw_el(|raw_el| {
                raw_el.on_signal_with_system(focus_signal, |In((entity, focus)), mut focused_option: ResMut<InputFocus>, mut commands: Commands| {
                    if focus {
                        focused_option.0 = Some(entity);
                    } else if let Some(focused) = focused_option.0 {
                        if focused == entity {
                            focused_option.0 = None;
                        }
                    }
                })
            })
        }
        self
    }

    /// When the string in this input changes, run a `handler` [`System`] which takes [`In`](System::In) the [`Entity`] of this input's [`Entity`] and the new [`String`].
    pub fn on_change_with_system<Marker>(self, handler: impl IntoSystem<In<(Entity, String,)>, (), Marker> + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Arc::new(OnceLock::new());
            raw_el.on_spawn(clone!((system_holder) move |world, entity| {
                let system = register_system(world, handler);
                let _ = system_holder.set(system);
                observe(world, entity, move |change: Trigger<TextInputChange>, mut commands: Commands| {
                    commands.run_system_with(system, (change.target(), change.event().0.clone()));
                });
            }))
            .with_entity(|mut entity| { entity.insert_if_new((ListenToChanges, TextInputContents::default())); })
            .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When the text of this input changes, run a function with the new text.
    pub fn on_change(self, mut handler: impl FnMut(String) + Send + Sync + 'static) -> Self {
        self.on_change_with_system(move |In((_, text))| handler(text))
    }

    /// Sync a [`Mutable`] with the text of this input.
    pub fn on_change_sync(self, string: Mutable<String>) -> Self {
        self.on_change(move |text| string.set_neq(text))
    }
}

#[derive(Component)]
struct ListenToChanges;

#[derive(Event)]
struct TextInputChange(String);

fn on_change(contents: Query<(Entity, &TextInputContents), (Changed<TextInputContents>, With<ListenToChanges>)>, mut commands: Commands) {
    for (entity, contents) in contents.iter() {
        commands.trigger_targets(TextInputChange(contents.get().to_string()), entity);
    }
}

#[derive(Event)]
struct FocusedChange(bool);

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
                commands.trigger_targets(FocusedChange(true), entity);
            }
        } else if focusable.is_focused {
            focusable.is_focused = false;
            commands.trigger_targets(FocusedChange(false), entity);
        }
    }
}

impl_haalka_methods! {
    TextInput {
        node: Node,
        text_input_buffer: TextInputBuffer,
        text_font: TextFont,
        text_input_layout_info: TextInputLayoutInfo,
        text_input_style: TextInputStyle,
        text_color: TextColor,
        text_input_prompt: TextInputPrompt,
    }
}

pub(super) fn plugin(app: &mut App) {
    app
    .add_plugins(TextInputPlugin)
    .add_systems(
        Update,
        (
            on_change.run_if(any_with_component::<ListenToChanges>),
            on_focus_changed.run_if(resource_changed_or_removed::<InputFocus>)
        )
            .run_if(any_with_component::<TextInputNode>),
    );
}
