//! Reactive text input widget and adjacent utilities, a thin wrapper around [`bevy_cosmic_edit`] integrated with [`Signal`]s.

use std::{ops::{Deref, Not}, pin::Pin};

use bevy_ecs::system::*;
use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_color::prelude::*;
use bevy_utils::prelude::*;
use bevy_app::prelude::*;
use bevy_derive::*;
use bevy_picking::prelude::*;
use bevy_text::cosmic_text;

use crate::impl_haalka_methods;

use super::{
    el::El, element::{ElementWrapper, Nameable, UiRootable}, pointer_event_aware::{PointerEventAware, CursorOnHoverable}, raw::{RawElWrapper, register_system}, mouse_wheel_scrollable::MouseWheelScrollable,
    sizeable::Sizeable, utils::clone, viewport_mutable::ViewportMutable, global_event_aware::GlobalEventAware,
    raw::{observe, utils::remove_system_holder_on_remove}
};
use apply::Apply;
use bevy_cosmic_edit::{self, *, prelude::*, FocusedWidget as CosmicFocusedWidget};
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

/// Marker [`Component`] for [`TextInput`] to prevent focusing on [`Pointer<Down>`] events. Useful when input focus is more conditional.
#[derive(Component)]
pub struct TextInputFocusOnDownDisabled;

// TODO: allow managing multiple spans reactively
impl TextInput {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        let el = El::<Node>::new().update_raw_el(|raw_el| {
            raw_el
                .insert((TextEdit, PickingBehavior::default()))
                .on_event_with_system::<Pointer<Down>, _>(
                    move |In((_, pointer_down)): In<(_, Pointer<Down>)>,
                            mut focusable_query: Query<(Entity, &mut Focusable), Without<TextInputFocusOnDownDisabled>>,
                            mut commands: Commands| {
                        // TODO: remove this focusable trigger and uncomment .insert_resource below when https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
                        // otherwise cursor position is not instantly correct on `Down`
                        if let Ok((entity, mut focusable)) = focusable_query.get_mut(pointer_down.target) {
                            focusable.is_focused = true;
                            commands.trigger_targets(FocusedChange(true), entity);
                        }
                        // commands.insert_resource(CosmicFocusedWidget(cosmic_edit_holder.get()));
                    },
                )
        });
        Self { el }
    }

    // /// Run a function with this input's [`CosmicEditBundle`]'s [`EntityWorldMut`].
    // pub fn with_cosmic_edit(self, f: impl FnOnce(EntityWorldMut) + Send + 'static) -> Self {
    //     self.update_raw_el(|raw_el| raw_el.with_entity_forwarded(cosmic_edit_entity_forwarder, f))
    // }

    // /// Add a [`Bundle`] of components to this input's [`CosmicEditBundle`] entity.
    // pub fn cosmic_edit_insert<B: Bundle>(self, bundle: B) -> Self {
    //     self.update_raw_el(|raw_el| raw_el.insert_forwarded(cosmic_edit_entity_forwarder, bundle))
    // }

    // /// Run a function with mutable access (via [`Mut`]) to this input's [`CosmicEditBundle`]'s entity's `C` [`Component`] if it exists.
    // pub fn with_cosmic_edit_component<C: Component>(self, f: impl FnOnce(Mut<C>) + Send + 'static) -> Self {
    //     self.update_raw_el(|raw_el| raw_el.with_component_forwarded(cosmic_edit_entity_forwarder, f))
    // }

    // /// Reactively run a function with this input's [`CosmicEditBundle`]'s [`EntityWorldMut`] and the output of the [`Signal`].
    // pub fn on_signal_with_cosmic_edit<T: Send + 'static>(
    //     self,
    //     signal: impl Signal<Item = T> + Send + 'static,
    //     f: impl FnMut(EntityWorldMut, T) + Send + Sync + 'static,
    // ) -> Self {
    //     self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(signal, f))
    // }

    // /// Reactively run a function with this input's [`CosmicEditBundle`]'s entity's `C` [`Component`] if it exists and the output of the [`Signal`].
    // pub fn on_signal_with_cosmic_edit_component<T: Send + 'static, C: Component>(
    //     self,
    //     signal: impl Signal<Item = T> + Send + 'static,
    //     f: impl FnMut(Mut<C>, T) + Send + Sync + 'static,
    // ) -> Self {
    //     self.update_raw_el(|raw_el| raw_el.on_signal_with_component(signal, f))
    // }

    // /// Reactively set this input's [`CosmicEditBundle`]'s entity's `C` [`Component`]. If the [`Signal`] outputs [`None`], the `C` [`Component`] is removed.
    // pub fn cosmic_edit_component_signal<C: Component, S: Signal<Item = impl Into<Option<C>>> + Send + 'static>(
    //     mut self,
    //     component_option_signal_option: impl Into<Option<S>>,
    // ) -> Self {
    //     if let Some(component_option_signal) = component_option_signal_option.into() {
    //         self = self.update_raw_el(|raw_el| {
    //             raw_el.component_signal(component_option_signal)
    //         });
    //     }
    //     self
    // }

    /// Run a function with this input's [`CosmicBuffer`] with access to [`ResMut<CosmicFontSystem>`] and [`DefaultAttrs`].
    pub fn with_cosmic_buffer(
        self,
        f: impl FnOnce(Mut<CosmicEditBuffer>, ResMut<CosmicFontSystem>, &DefaultAttrs) + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.with_entity(move |mut entity| {
            let id = entity.id();
            entity.world_scope(|world| {
                // TODO: is this stuff repeated for every call ?
                #[allow(clippy::type_complexity)]
                let mut system_state: SystemState<(
                    ResMut<CosmicFontSystem>,
                    Query<(&mut CosmicEditBuffer, &DefaultAttrs)>,
                )> = SystemState::new(world);
                let (font_system, mut cosmic_buffer_query) = system_state.get_mut(world);
                let Ok((cosmic_buffer, attrs)) = cosmic_buffer_query.get_mut(id) else {
                    return;
                };
                f(cosmic_buffer, font_system, attrs)
            });
        }))
    }

    /// Reactively run a function with this input's [`CosmicBuffer`] and the output of the [`Signal`] with access to [`ResMut<CosmicFontSystem>`] and [`DefaultAttrs`].
    pub fn on_signal_with_cosmic_buffer<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Mut<CosmicEditBuffer>, ResMut<CosmicFontSystem>, &DefaultAttrs, T) + Send + Sync + 'static,
    ) -> Self {
        self.update_raw_el(move |raw_el| {
            raw_el.on_signal_one_shot(
                signal,
                move |In((entity, value)): In<(Entity, T)>,
                      font_system: ResMut<CosmicFontSystem>,
                      mut cosmic_buffer_query: Query<(&mut CosmicEditBuffer, &DefaultAttrs)>| {
                    let Ok((cosmic_buffer, attrs)) = cosmic_buffer_query.get_mut(entity) else {
                        return;
                    };
                    f(cosmic_buffer, font_system, attrs, value)
                },
            )
        })
    }

    /// Set the text of this input.
    pub fn text(mut self, text_option: impl Into<Option<String>>) -> Self {
        if let Some(text) = text_option.into() {
            self = self.with_cosmic_buffer(move |mut cosmic_buffer, mut font_system, attrs| {
                cosmic_buffer.set_text(&mut font_system, &text, attrs.0.as_attrs());
            })
        }
        self
    }

    /// Reactively set the text of this input. If the signal outputs [`None`] the text is set to an empty string.
    pub fn text_signal<S: Signal<Item = impl Into<Option<String>>> + Send + 'static>(
        mut self,
        text_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_option_signal) = text_option_signal_option.into() {
            self = self.on_signal_with_cosmic_buffer(
                text_option_signal.map(|text_option| text_option.into()),
                |mut cosmic_buffer, mut font_system, attrs, text_option| {
                    cosmic_buffer.set_text(&mut font_system, &text_option.unwrap_or_default(), attrs.0.as_attrs());
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
            let system_holder = Mutable::new(None);
            raw_el
            .with_entity(|mut entity| { entity.insert(Focusable { is_focused: false }); })
            .on_spawn(clone!((system_holder) move |world, entity| {
                let system = register_system(world, handler);
                system_holder.set(Some(system));
                observe(world, entity, move |event: Trigger<FocusedChange>, mut commands: Commands| {
                    commands.run_system_with_input(system, (entity, event.event().0))
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
                commands.insert_resource(FocusedTextInput(entity));
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
                raw_el.on_signal_one_shot(focus_signal, |In((entity, focus)), focused_option: Option<Res<FocusedTextInput>>, mut commands: Commands| {
                    if focus {
                        commands.insert_resource(FocusedTextInput(entity));
                    } else if let Some(focused) = focused_option {
                        if focused.0 == entity {
                            commands.remove_resource::<FocusedTextInput>();
                        }
                    }
                })
            })
        }
        self
    }

    /// Set the font size of this input.
    pub fn font_size(mut self, font_size_option: impl Into<Option<f32>>) -> Self {
        if let Some(font_size) = font_size_option.into() {
            self = self.with_cosmic_buffer(move |mut cosmic_buffer, mut font_system, _| {
                let mut metrics = cosmic_buffer.metrics();
                metrics.font_size = font_size;
                cosmic_buffer.set_metrics(&mut font_system, metrics);
            })
        }
        self
    }

    /// Reactively set the font size of this input.
    pub fn font_size_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        font_size_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(font_size_signal) = font_size_signal_option.into() {
            self = self.on_signal_with_cosmic_buffer(font_size_signal, |mut cosmic_buffer, mut font_system, _, font_size| {
                let mut metrics = cosmic_buffer.metrics();
                metrics.font_size = font_size;
                cosmic_buffer.set_metrics(&mut font_system, metrics);
            });
        }
        self
    }

    /// Set the line height of this input.
    pub fn line_height(mut self, line_height_option: impl Into<Option<f32>>) -> Self {
        if let Some(line_height) = line_height_option.into() {
            self = self.with_cosmic_buffer(move |mut cosmic_buffer, mut font_system, _| {
                let mut metrics = cosmic_buffer.metrics();
                metrics.line_height = line_height;
                cosmic_buffer.set_metrics(&mut font_system, metrics);
            })
        }
        self
    }

    /// Reactively set the line height of this input.
    pub fn line_height_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        line_height_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(line_height_signal) = line_height_signal_option.into() {
            self =
                self.on_signal_with_cosmic_buffer(line_height_signal, |mut cosmic_buffer, mut font_system, _, line_height| {
                    let mut metrics = cosmic_buffer.metrics();
                    metrics.line_height = line_height;
                    cosmic_buffer.set_metrics(&mut font_system, metrics);
                });
        }
        self
    }

    /// Set the text attributes of this input.
    pub fn attrs(mut self, attrs_option: impl Into<Option<TextAttrs>>) -> Self {
        // TODO: happened to want the type hinting here, but should be able to use .into() like i do everywhere else https://github.com/rust-lang/rust-analyzer/issues/17307
        if let Some(attrs) = Into::<Option<TextAttrs>>::into(attrs_option) {
            if let Some(color_signal) = attrs.color_opt {
                let color = color_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     color.signal(),
                    //     move |mut attrs, color_option| {
                    //         attrs.color_opt = color_option;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(color.signal(), |mut cosmic_buffer, mut font_system, attrs, color_option| {
                        let mut attrs = attrs.0.clone();
                        attrs.color_opt = color_option;
                        set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs);
                    });
            }
            if let Some(family_signal) = attrs.family_owned {
                let family = family_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     family.signal_cloned(),
                    //     move |mut attrs, family| {
                    //         attrs.family_owned = family;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(
                        family.signal_cloned(),
                        |mut cosmic_buffer, mut font_system, attrs, family| {
                            let mut attrs = attrs.0.clone();
                            attrs.family_owned = family;
                            set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs)
                        },
                    )
            }
            if let Some(stretch_signal) = attrs.stretch {
                let stretch = stretch_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     stretch.signal(),
                    //     move |mut attrs, stretch| {
                    //         attrs.stretch = stretch;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(stretch.signal(), |mut cosmic_buffer, mut font_system, attrs, stretch| {
                        let mut attrs = attrs.0.clone();
                        attrs.stretch = stretch;
                        set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs)
                    })
            }
            if let Some(style_signal) = attrs.style {
                let style = style_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     node.signal(),
                    //     move |mut attrs, style| {
                    //         attrs.style = style;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(style.signal(), |mut cosmic_buffer, mut font_system, attrs, style| {
                        let mut attrs = attrs.0.clone();
                        attrs.style = style;
                        set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs)
                    })
            }
            if let Some(weight_signal) = attrs.weight {
                let weight = weight_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     weight.signal(),
                    //     move |mut attrs, weight| {
                    //         attrs.weight = weight;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(weight.signal(), |mut cosmic_buffer, mut font_system, attrs, weight| {
                        let mut attrs = attrs.0.clone();
                        attrs.weight = weight;
                        set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs)
                    })
            }
            if let Some(metadata_signal) = attrs.metadata {
                let metadata = metadata_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     metadata.signal(),
                    //     move |mut attrs, metadata| {
                    //         attrs.metadata = metadata;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(metadata.signal(), |mut cosmic_buffer, mut font_system, attrs, metadata| {
                        let mut attrs = attrs.0.clone();
                        attrs.metadata = metadata;
                        set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs)
                    })
            }
            if let Some(cache_key_flags_signal) = attrs.cache_key_flags {
                let cache_key_flags = cache_key_flags_signal.broadcast();
                self = self
                    // .on_signal_with_default_attrs(
                    //     cache_key_flags.signal(),
                    //     move |mut attrs, cache_key_flags| {
                    //         attrs.cache_key_flags = cache_key_flags;
                    //     },
                    // )
                    .on_signal_with_cosmic_buffer(
                        cache_key_flags.signal(),
                        |mut cosmic_buffer, mut font_system, attrs, cache_key_flags| {
                            let mut attrs = attrs.0.clone();
                            attrs.cache_key_flags = cache_key_flags;
                            set_text_attrs(&mut cosmic_buffer, &mut font_system, attrs)
                        },
                    )
            }
        }
        self
    }

    /// Add a [`Component`] with [`Default`] to this input.
    pub fn unit_component<C: Component + Default>(mut self, option: impl Into<Option<bool>>) -> Self {
        if Into::<Option<bool>>::into(option).unwrap_or(false) {
            self = self.update_raw_el(|raw_el| raw_el.insert(C::default()));
        }
        self
    }

    /// Reactively set a [`Component`] with [`Default`] to this input. If the [`Signal`] outputs `false`, the `C` [`Component`] is removed.
    pub fn cosmic_edit_unit_component_signal<C: Component + Default, S: Signal<Item = bool> + Send + 'static>(
        mut self,
        component_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(component_option_signal) = component_option_signal_option.into() {
            self = self.update_raw_el(|raw_el| raw_el.component_signal::<C, _>(component_option_signal.map_true(C::default)));
        }
        self
    }

    /// Set whether the user is prevented from editing the text of this input.
    pub fn read_only_option(self, read_only_option: impl Into<Option<bool>>) -> Self {
        self.unit_component::<bevy_cosmic_edit::ReadOnly>(read_only_option)
    }

    /// Prevent the user from editing the text of this input.
    pub fn read_only(self) -> Self {
        self.read_only_option(true)
    }

    /// Reactively set whether the user is prevented from editing the text of this input.
    pub fn read_only_signal<S: Signal<Item = bool> + Send + 'static>(
        self,
        read_only_signal_option: impl Into<Option<S>>,
    ) -> Self {
        self.cosmic_edit_unit_component_signal::<bevy_cosmic_edit::ReadOnly, _>(read_only_signal_option)
    }

    /// Set whether the user is prevented from scrolling the text of this input.
    pub fn scroll_enabled_option(self, scroll_enabled_option: impl Into<Option<bool>>) -> Self {
        self.unit_component::<bevy_cosmic_edit::ScrollEnabled>(scroll_enabled_option)
    }

    /// Prevent the user from scrolling the text of this input.
    pub fn scroll_enabled(self) -> Self {
        self.scroll_enabled_option(true)
    }

    /// Reactively set whether the user is prevented from scrolling the text of this input.
    pub fn scroll_enabled_signal<S: Signal<Item = bool> + Send + 'static>(
        self,
        scroll_enabled_signal_option: impl Into<Option<S>>,
    ) -> Self {
        self.cosmic_edit_unit_component_signal::<bevy_cosmic_edit::ScrollEnabled, _>(scroll_enabled_signal_option)
    }

    /// Set whether the user is prevented from selecting the text of this input.
    pub fn user_select_none_option(self, user_select_none_option: impl Into<Option<bool>>) -> Self {
        self.unit_component::<bevy_cosmic_edit::UserSelectNone>(user_select_none_option)
    }

    /// Prevent the user from selecting the text of this input.
    pub fn user_select_none(self) -> Self {
        self.user_select_none_option(true)
    }

    /// Reactively set whether the user is prevented from selecting the text of this input.
    pub fn user_select_none_signal<S: Signal<Item = bool> + Send + 'static>(
        self,
        user_select_none_signal_option: impl Into<Option<S>>,
    ) -> Self {
        self.cosmic_edit_unit_component_signal::<bevy_cosmic_edit::UserSelectNone, _>(user_select_none_signal_option)
    }

    /// Set the placeholder of this input.
    pub fn placeholder(mut self, placeholder_option: impl Into<Option<Placeholder>>) -> Self {
        if let Some(placeholder) = Into::<Option<Placeholder>>::into(placeholder_option) {
            if let Some(text_signal) = placeholder.text {
                self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(text_signal, move |mut entity, text| {
                    if let Some(mut placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                        placeholder.text = text;
                    } else {
                        entity.insert(bevy_cosmic_edit::Placeholder::new(text, cosmic_text::Attrs::new()));
                    }
                }));
            }
            if let Some(attrs) = placeholder.attrs {
                if let Some(color_signal) = attrs.color_opt {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(color_signal, move |mut entity, color_option| {
                        if let Some(mut placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.color_opt = color_option;
                        } else {
                            let mut attrs = cosmic_text::Attrs::new();
                            attrs.color_opt = color_option;
                            entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    }));
                }
                if let Some(family_signal) = attrs.family_owned {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(family_signal, move |mut entity, family| {
                        if let Some(placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.family(family.as_family());
                        } else {
                            let attrs = cosmic_text::Attrs::new();
                            attrs.family(family.as_family());
                            entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    }));
                }
                if let Some(stretch_signal) = attrs.stretch {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(stretch_signal, move |mut entity, stretch| {
                        if let Some(placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.stretch(stretch);
                        } else {
                            let attrs = cosmic_text::Attrs::new();
                            attrs.stretch(stretch);
                            entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    }));
                }
                if let Some(style_signal) = attrs.style {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(style_signal, move |mut entity, style| {
                        if let Some(placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.style(style);
                        } else {
                            let attrs = cosmic_text::Attrs::new();
                            attrs.style(style);
                            entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    }));
                }
                if let Some(weight_signal) = attrs.weight {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(weight_signal, move |mut entity, weight| {
                        if let Some(placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.weight(weight);
                        } else {
                            let attrs = cosmic_text::Attrs::new();
                            attrs.weight(weight);
                            entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    }));
                }
                if let Some(metadata_signal) = attrs.metadata {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(metadata_signal, move |mut entity, metadata| {
                        if let Some(mut placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.metadata = metadata;
                        } else {
                            let mut attrs = cosmic_text::Attrs::new();
                            attrs.metadata = metadata;
                            entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    }));
                }
                if let Some(cache_key_flags_signal) = attrs.cache_key_flags {
                    self = self.update_raw_el(|raw_el| raw_el.on_signal_with_entity(
                        cache_key_flags_signal,
                        move |mut entity, cache_key_flags| {
                            if let Some(placeholder) = entity.get_mut::<bevy_cosmic_edit::Placeholder>() {
                                placeholder.attrs.cache_key_flags(cache_key_flags);
                            } else {
                                let attrs = cosmic_text::Attrs::new();
                                attrs.cache_key_flags(cache_key_flags);
                                entity.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                            }
                        },
                    ));
                }
            }
        }
        self
    }

    /// When the string in this input changes, run a `handler` [`System`] which takes [`In`](System::In) the [`Entity`] of this input's [`Entity`] and the new [`String`].
    pub fn on_change_with_system<Marker>(self, handler: impl IntoSystem<In<(Entity, String,)>, (), Marker> + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            let system_holder = Mutable::new(None);
            raw_el.on_spawn(clone!((system_holder) move |world, entity| {
                let system = register_system(world, handler);
                system_holder.set(Some(system));
                observe(world, entity, move |change: Trigger<TextInputChange>, mut commands: Commands| {
                    let entity = change.entity();
                    commands.run_system_with_input(system, (entity, change.event().0.clone()));
                });
            }))
            .insert(ListeningToChanges)
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
struct ListeningToChanges;

fn set_text_attrs(cosmic_buffer: &mut CosmicEditBuffer, font_system: &mut FontSystem, attrs: cosmic_text::AttrsOwned) {
    let spans = cosmic_buffer.get_text_spans(attrs.clone());
    if let Some(list_spans) = spans.first() {
        if let Some((text, _)) = list_spans.first() {
            cosmic_buffer.set_text(font_system, text, attrs.as_attrs());
        }
    }
}

#[derive(Event)]
struct TextInputChange(String);

fn on_change(mut changed_events: EventReader<CosmicTextChanged>, mut commands: Commands) {
    for CosmicTextChanged((entity, text)) in changed_events.read() {
        commands.trigger_targets(TextInputChange(text.clone()), *entity);
    }
}

#[derive(Event)]
struct FocusedChange(bool);

#[derive(Component)]
struct Focusable {
    is_focused: bool,
}

/// [`Resource`] holding the [`Entity`] of the currently focused [`TextInput`]; if one isn't
/// focused, this resource won't exist in the [`World`]. This resource can be added, mutated, or
/// removed to control the focused [`TextInput`].
#[derive(Resource, Deref)]
pub struct FocusedTextInput(pub Entity);

fn sync_cosmic_focus(
    focused_widget: Res<CosmicFocusedWidget>,
    mut commands: Commands,
) {
    if let Some(entity) = focused_widget.0 {
        commands.insert_resource(FocusedTextInput(entity));
    } else {
        commands.remove_resource::<FocusedTextInput>();
    }

}

fn on_focus_changed(
    focused_option: Option<Res<FocusedTextInput>>,
    mut text_inputs: Query<(Entity, &mut Focusable)>,
    mut cosmic_focused_widget: ResMut<CosmicFocusedWidget>,
    mut commands: Commands,
) {
    let focused_option = focused_option.as_deref().map(Deref::deref).copied();
    for (entity, mut focusable) in text_inputs.iter_mut() {
        if Some(entity) == focused_option {
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
    if let Some(focused) = focused_option {
        if cosmic_focused_widget.0 != Some(focused) {
            cosmic_focused_widget.0 = Some(focused);
        }
    } else if cosmic_focused_widget.0.is_some() {
        cosmic_focused_widget.0 = None;
    }
}

/// An owned dynamically typed [`Signal`] which is both [`Send`] and [`Sync`].
pub type BoxSignalSync<'a, T> = Pin<Box<dyn Signal<Item = T> + Send + Sync + 'a>>;

/// Allows setting the text attributes of a [`TextInput`] and its [placeholder](`TextInput::placeholder`). These settings can be either static or reactive via [`Signal`]s. See [`cosmic_text::AttrsOwned`].
#[derive(Default)]
pub struct TextAttrs {
    color_opt: Option<BoxSignalSync<'static, Option<CosmicColor>>>,
    family_owned: Option<BoxSignalSync<'static, cosmic_text::FamilyOwned>>,
    stretch: Option<BoxSignalSync<'static, cosmic_text::Stretch>>,
    style: Option<BoxSignalSync<'static, FontStyle>>,
    weight: Option<BoxSignalSync<'static, FontWeight>>,
    metadata: Option<BoxSignalSync<'static, usize>>,
    cache_key_flags: Option<BoxSignalSync<'static, cosmic_text::CacheKeyFlags>>,
}

impl TextAttrs {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        default()
        // .family(FamilyOwned::new(bevy_cosmic_edit::Family::Name("Fira Mono")))
    }

    /// Reactively set the color of this text. If the signal outputs [`None`] the color is set to its default white.
    pub fn color_signal<C: Into<Color>, S: Signal<Item = Option<C>> + Send + Sync + 'static>(
        mut self,
        color_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(color_signal) = color_signal_option.into() {
            self.color_opt = Some(
                color_signal
                    .map(|color_option| color_option.map(bevy_cosmic_edit::utils::ColorExtras::to_cosmic))
                    .apply(Box::pin),
            );
        }
        self
    }

    /// Set the color of this text.
    pub fn color<C: Into<Color>>(mut self, color_option: impl Into<Option<C>>) -> Self {
        if let Some(color) = color_option.into() {
            self = self.color_signal(always(Some(color.into())));
        }
        self
    }

    /// Reactively set the font family of this text.
    pub fn family_signal<S: Signal<Item = cosmic_text::FamilyOwned> + Send + Sync + 'static>(
        mut self,
        family_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(family_signal) = family_signal_option.into() {
            self.family_owned = Some(Box::pin(family_signal));
        }
        self
    }

    /// Set the font family of this text.
    pub fn family(mut self, family_option: impl Into<Option<cosmic_text::FamilyOwned>>) -> Self {
        if let Some(family) = family_option.into() {
            self = self.family_signal(always(family));
        }
        self
    }

    /// Reactively set the stretch of this text.
    pub fn stretch_signal<S: Signal<Item = cosmic_text::Stretch> + Send + Sync + 'static>(
        mut self,
        stretch_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(stretch_signal) = stretch_signal_option.into() {
            self.stretch = Some(Box::pin(stretch_signal));
        }
        self
    }

    /// Set the stretch of this text.
    pub fn stretch(mut self, stretch_option: impl Into<Option<cosmic_text::Stretch>>) -> Self {
        if let Some(stretch) = stretch_option.into() {
            self = self.stretch_signal(always(stretch));
        }
        self
    }

    /// Reactively set the font style of this text.
    pub fn style_signal<S: Signal<Item = FontStyle> + Send + Sync + 'static>(
        mut self,
        style_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(style_signal) = style_signal_option.into() {
            self.style = Some(Box::pin(style_signal));
        }
        self
    }

    /// Set the font style of this text.
    pub fn style(mut self, style_option: impl Into<Option<FontStyle>>) -> Self {
        if let Some(style) = style_option.into() {
            self = self.style_signal(always(style));
        }
        self
    }

    /// Reactively set the font weight of this text.
    pub fn weight_signal<S: Signal<Item = FontWeight> + Send + Sync + 'static>(
        mut self,
        weight_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(weight_signal) = weight_signal_option.into() {
            self.weight = Some(Box::pin(weight_signal));
        }
        self
    }

    /// Set the font weight of this text.
    pub fn weight(mut self, weight_option: impl Into<Option<FontWeight>>) -> Self {
        if let Some(weight) = weight_option.into() {
            self = self.weight_signal(always(weight));
        }
        self
    }

    /// Reactively set the metadata of this text.
    pub fn metadata_signal<S: Signal<Item = usize> + Send + Sync + 'static>(
        mut self,
        metadata_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(metadata_signal) = metadata_signal_option.into() {
            self.metadata = Some(Box::pin(metadata_signal));
        }
        self
    }

    /// Set the metadata of this text.
    pub fn metadata(mut self, metadata_option: impl Into<Option<usize>>) -> Self {
        if let Some(metadata) = metadata_option.into() {
            self = self.metadata_signal(always(metadata));
        }
        self
    }

    /// Reactively set the cache key flags of this text.
    pub fn cache_key_flags_signal<S: Signal<Item = cosmic_text::CacheKeyFlags> + Send + Sync + 'static>(
        mut self,
        cache_key_flags_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cache_key_flags_signal) = cache_key_flags_signal_option.into() {
            self.cache_key_flags = Some(Box::pin(cache_key_flags_signal));
        }
        self
    }

    /// Set the cache key flags of this text.
    pub fn cache_key_flags(
        mut self,
        cache_key_flags_option: impl Into<Option<cosmic_text::CacheKeyFlags>>,
    ) -> Self {
        if let Some(cache_key_flags) = cache_key_flags_option.into() {
            self = self.cache_key_flags_signal(always(cache_key_flags));
        }
        self
    }
}

/// A placeholder for a [`TextInput`]. The text and text attributes can be either static or reactive via [`Signal`]s.
#[derive(Default)]
pub struct Placeholder {
    text: Option<BoxSignal<'static, &'static str>>,
    attrs: Option<TextAttrs>,
}

impl Placeholder {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        default()
    }

    /// Reactively set the text of this placeholder. If the signal outputs [`None`] the text is set to an empty string.
    pub fn text_signal<S: Signal<Item = &'static str> + Send + 'static>(
        mut self,
        text_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_signal) = text_signal_option.into() {
            self.text = Some(Box::pin(text_signal));
        }
        self
    }

    /// Set the text of this placeholder.
    pub fn text(mut self, text_option: impl Into<Option<&'static str>>) -> Self {
        if let Some(text) = text_option.into() {
            self = self.text_signal(always(text));
        }
        self
    }

    /// Set the text attributes of this placeholder.
    pub fn attrs(mut self, attrs_option: impl Into<Option<TextAttrs>>) -> Self {
        self.attrs = attrs_option.into();
        self
    }
}

impl_haalka_methods! {
    TextInput {
        buffer: CosmicEditBuffer,
        fill_color: bevy_cosmic_edit::CosmicBackgroundColor,
        cursor_color: bevy_cosmic_edit::CursorColor,
        selection_color: bevy_cosmic_edit::SelectionColor,
        default_attrs: bevy_cosmic_edit::DefaultAttrs,
        background_image: bevy_cosmic_edit::CosmicBackgroundImage,
        max_lines: bevy_cosmic_edit::MaxLines,
        max_chars: bevy_cosmic_edit::MaxChars,
        mode: bevy_cosmic_edit::CosmicWrap,
        text_position: bevy_cosmic_edit::CosmicTextAlign,
        hover_cursor: bevy_cosmic_edit::HoverCursor,
    }
}

pub(super) fn plugin(app: &mut App) {
    // TODO: this requires including the assets in the crate but we want to avoid that
    let font_bytes: &[u8] = include_bytes!("../assets/fonts/FiraMono-subset.ttf");
    let font_config = bevy_cosmic_edit::CosmicFontConfig {
        fonts_dir_path: None,
        font_bytes: Some(vec![font_bytes]),
        load_system_fonts: true,
    };
    app
    .add_plugins(bevy_cosmic_edit::CosmicEditPlugin { font_config })
    .add_systems(
        Update,
        (
            on_change.run_if(any_with_component::<ListeningToChanges>.and(on_event::<CosmicTextChanged>)),
            (
                sync_cosmic_focus.run_if(resource_changed::<CosmicFocusedWidget>),
                on_focus_changed.run_if(resource_changed_or_removed::<FocusedTextInput>)
            ).chain(),
        )
            .run_if(any_with_component::<TextEdit>),
    );
}
