use std::{ops::Not, pin::Pin};

use bevy::{
    ecs::system::{SystemId, SystemState},
    prelude::*,
};
use bevy_eventlistener::callbacks::Listener;
use bevy_mod_picking::{
    events::{Down, Pointer},
    picking_core::Pickable,
};

use crate::{clone, El, ElementWrapper, PointerEventAware, RawElWrapper, Scrollable, Sizeable, ViewportMutable};
use apply::Apply;
use bevy_cosmic_edit::{
    self, ColorExtras, CosmicBuffer, CosmicColor, CosmicEditBundle, CosmicFontSystem, CosmicSource, CosmicTextChanged,
    DefaultAttrs, FocusedWidget as CosmicFocusedWidget, FontSystem,
};
use futures_signals::signal::{always, BoxSignal, Mutable, Signal, SignalExt};
use futures_signals_ext::SignalExtBool;

pub struct TextInput {
    el: El<ButtonBundle>,
}

impl ElementWrapper for TextInput {
    type EL = El<ButtonBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl PointerEventAware for TextInput {}
impl Scrollable for TextInput {}
impl Sizeable for TextInput {}
impl ViewportMutable for TextInput {}

// TODO: allow managing multiple spans reactively
impl TextInput {
    pub fn new() -> Self {
        let cosmic_edit_holder = Mutable::new(None);
        let el = El::<ButtonBundle>::new().update_raw_el(|raw_el| {
            raw_el
                .with_entity(clone!((cosmic_edit_holder) move |mut entity| {
                    let cosmic_edit = entity.world_scope(|world| world.spawn(CosmicEditBundle::default()).id());
                    cosmic_edit_holder.set(Some(cosmic_edit));
                    entity
                        // TODO: once bevy 0.14, can just add an OnRemove to despawn the corresponding cosmic edit entity to
                        // avoid the non ui child warning
                        .add_child(cosmic_edit)
                        .insert(CosmicSource(cosmic_edit));
                }))
                .update_raw_el(|raw_el| {
                    raw_el
                        .insert(Pickable::default())
                        .on_event_with_system::<Pointer<Down>, _>(
                            move |pointer_down: Listener<Pointer<Down>>,
                                  mut focusable_query: Query<&mut Focusable>,
                                  mut commands: Commands| {
                                // TODO: remove this focusable trigger and uncomment .insert_resource below when https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
                                // otherwise cursor position is not instantly correct on `Down`
                                if let Ok(mut focusable) = focusable_query.get_mut(pointer_down.target) {
                                    focusable.is_focused = true;
                                    commands.run_system_with_input(focusable.system, (true,));
                                }
                                // commands.insert_resource(CosmicFocusedWidget(cosmic_edit_holder.
                                // get()));
                            },
                        )
                })
        });
        Self { el }
    }

    pub fn with_cosmic_edit(self, f: impl FnOnce(EntityWorldMut) + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.with_entity_forwarded(cosmic_edit_entity_forwarder, f))
    }

    pub fn cosmic_edit_insert<B: Bundle>(self, bundle: B) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert_forwarded(cosmic_edit_entity_forwarder, bundle))
    }

    pub fn with_cosmic_edit_component<C: Component>(self, f: impl FnOnce(&mut C) + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| raw_el.with_component_forwarded(cosmic_edit_entity_forwarder, f))
    }

    pub fn on_signal_with_cosmic_edit<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        f: impl FnMut(EntityWorldMut, T) + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.on_signal_with_entity_forwarded(signal, cosmic_edit_entity_forwarder, f))
    }

    pub fn on_signal_with_cosmic_edit_component<T: Send + 'static, C: Component>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        f: impl FnMut(&mut C, T) + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.on_signal_with_component_forwarded(signal, cosmic_edit_entity_forwarder, f))
    }

    pub fn cosmic_edit_component_signal<C: Component, S: Signal<Item = impl Into<Option<C>>> + Send + 'static>(
        mut self,
        component_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(component_option_signal) = component_option_signal_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.component_signal_forwarded(cosmic_edit_entity_forwarder, component_option_signal)
            });
        }
        self
    }

    pub fn with_cosmic_buffer(
        self,
        f: impl FnOnce(&mut CosmicBuffer, &mut FontSystem, &DefaultAttrs) + Send + 'static,
    ) -> Self {
        self.with_cosmic_edit(move |mut entity| {
            let id = entity.id();
            entity.world_scope(|world| {
                // TODO: is this stuff repeated for every call ?
                let mut system_state: SystemState<(
                    ResMut<CosmicFontSystem>,
                    Query<(&mut CosmicBuffer, &DefaultAttrs)>,
                )> = SystemState::new(world);
                let (mut font_system, mut cosmic_buffer_query) = system_state.get_mut(world);
                let Ok((mut cosmic_buffer, attrs)) = cosmic_buffer_query.get_mut(id) else {
                    return;
                };
                f(&mut cosmic_buffer, &mut font_system, attrs)
            });
        })
    }

    pub fn on_signal_with_cosmic_buffer<T: Send + 'static>(
        self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(&mut CosmicBuffer, &mut FontSystem, &DefaultAttrs, T) + Send + Sync + 'static,
    ) -> Self {
        self.update_raw_el(move |raw_el| {
            raw_el.on_signal_one_shot(
                signal,
                move |In((entity, value)): In<(Entity, T)>,
                      mut font_system: ResMut<CosmicFontSystem>,
                      cosmic_source_query: Query<&CosmicSource>,
                      mut cosmic_buffer_query: Query<(&mut CosmicBuffer, &DefaultAttrs)>| {
                    let Ok(cosmic_edit_entity) = cosmic_source_query
                        .get(entity)
                        .map(|&CosmicSource(cosmic_source)| cosmic_source)
                    else {
                        return;
                    };
                    let Ok((mut cosmic_buffer, attrs)) = cosmic_buffer_query.get_mut(cosmic_edit_entity) else {
                        return;
                    };
                    f(&mut cosmic_buffer, &mut font_system, attrs, value)
                },
            )
        })
    }

    pub fn text(mut self, text_option: impl Into<Option<String>>) -> Self {
        if let Some(text) = text_option.into() {
            self = self.with_cosmic_buffer(move |cosmic_buffer, font_system, attrs| {
                cosmic_buffer.set_text(font_system, &text, attrs.0.as_attrs());
            })
        }
        self
    }

    pub fn text_signal<S: Signal<Item = impl Into<Option<String>>> + Send + 'static>(
        mut self,
        text_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_option_signal) = text_option_signal_option.into() {
            self = self.on_signal_with_cosmic_buffer(
                text_option_signal.map(|text_option| text_option.into()),
                |cosmic_buffer, font_system, attrs, text_option| {
                    cosmic_buffer.set_text(font_system, &text_option.unwrap_or_default(), attrs.0.as_attrs());
                },
            );
        }
        self
    }

    pub fn on_focused_change_with_system<Marker>(
        self,
        handler: impl IntoSystem<(bool,), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.with_entity(move |mut entity| {
                let system = entity.world_scope(|world| world.register_system(handler));
                entity.insert(Focusable {
                    system,
                    is_focused: false,
                });
            })
        })
    }

    pub fn on_focused_change(self, mut handler: impl FnMut(bool) + Send + Sync + 'static) -> Self {
        self.on_focused_change_with_system(move |In((is_focused,))| handler(is_focused))
    }

    pub fn focused_sync(self, focused: Mutable<bool>) -> Self {
        self.on_focused_change(move |is_focused| focused.set_neq(is_focused))
    }

    pub fn focus_option(mut self, focus_option: impl Into<Option<bool>>) -> Self {
        if Into::<Option<bool>>::into(focus_option).unwrap_or(false) {
            self = self.with_cosmic_edit(|mut cosmic_edit| {
                let entity = cosmic_edit.id();
                cosmic_edit.world_scope(|world| {
                    world.insert_resource(bevy_cosmic_edit::FocusedWidget(Some(entity)));
                })
            });
        }
        self
    }

    pub fn focus(self) -> Self {
        self.focus_option(true)
    }

    pub fn focus_signal<S: Signal<Item = bool> + Send + 'static>(
        mut self,
        focus_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(focus_signal) = focus_signal_option.into() {
            self = self.on_signal_with_cosmic_edit(focus_signal, |mut cosmic_edit, focus| {
                let entity = cosmic_edit.id();
                cosmic_edit.world_scope(|world| {
                    if let Some(mut focused_widget) = world.get_resource_mut::<bevy_cosmic_edit::FocusedWidget>() {
                        if focus {
                            // TODO: does this actually not trigger change detection ?
                            if focused_widget.0 != Some(entity) {
                                focused_widget.0 = Some(entity);
                            }
                        } else if focused_widget.0 == Some(entity) {
                            focused_widget.0 = None;
                        }
                    } else if focus {
                        world.insert_resource(bevy_cosmic_edit::FocusedWidget(Some(entity)));
                    }
                })
            });
        }
        self
    }

    pub fn font_size(mut self, font_size_option: impl Into<Option<f32>>) -> Self {
        if let Some(font_size) = font_size_option.into() {
            self = self.with_cosmic_buffer(move |cosmic_buffer, font_system, _| {
                let mut metrics = cosmic_buffer.metrics();
                metrics.font_size = font_size;
                cosmic_buffer.set_metrics(font_system, metrics);
            })
        }
        self
    }

    pub fn font_size_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        font_size_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(font_size_signal) = font_size_signal_option.into() {
            self = self.on_signal_with_cosmic_buffer(font_size_signal, |cosmic_buffer, font_system, _, font_size| {
                let mut metrics = cosmic_buffer.metrics();
                metrics.font_size = font_size;
                cosmic_buffer.set_metrics(font_system, metrics);
            });
        }
        self
    }

    pub fn line_height(mut self, line_height_option: impl Into<Option<f32>>) -> Self {
        if let Some(line_height) = line_height_option.into() {
            self = self.with_cosmic_buffer(move |cosmic_buffer, font_system, _| {
                let mut metrics = cosmic_buffer.metrics();
                metrics.line_height = line_height;
                cosmic_buffer.set_metrics(font_system, metrics);
            })
        }
        self
    }

    pub fn line_height_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        line_height_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(line_height_signal) = line_height_signal_option.into() {
            self =
                self.on_signal_with_cosmic_buffer(line_height_signal, |cosmic_buffer, font_system, _, line_height| {
                    let mut metrics = cosmic_buffer.metrics();
                    metrics.line_height = line_height;
                    cosmic_buffer.set_metrics(font_system, metrics);
                });
        }
        self
    }

    pub fn attrs(mut self, attrs_option: impl Into<Option<TextAttrs>>) -> Self {
        // TODO: happened to want the type hinting here, but should be able to use .into() like i do everywhere else https://github.com/rust-lang/rust-analyzer/issues/17307
        if let Some(attrs) = Into::<Option<TextAttrs>>::into(attrs_option) {
            if let Some(color_signal) = attrs.color_opt {
                let color = color_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        color.signal(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), color_option| {
                            attrs.color_opt = color_option;
                        },
                    )
                    .on_signal_with_cosmic_buffer(color.signal(), |cosmic_buffer, font_system, attrs, color_option| {
                        let mut attrs = attrs.0.clone();
                        attrs.color_opt = color_option;
                        set_text_attrs(cosmic_buffer, font_system, attrs);
                    });
            }
            if let Some(family_signal) = attrs.family_owned {
                let family = family_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        family.signal_cloned(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), family| {
                            attrs.family_owned = family;
                        },
                    )
                    .on_signal_with_cosmic_buffer(
                        family.signal_cloned(),
                        |cosmic_buffer, font_system, attrs, family| {
                            let mut attrs = attrs.0.clone();
                            attrs.family_owned = family;
                            set_text_attrs(cosmic_buffer, font_system, attrs)
                        },
                    )
            }
            if let Some(stretch_signal) = attrs.stretch {
                let stretch = stretch_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        stretch.signal(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), stretch| {
                            attrs.stretch = stretch;
                        },
                    )
                    .on_signal_with_cosmic_buffer(stretch.signal(), |cosmic_buffer, font_system, attrs, stretch| {
                        let mut attrs = attrs.0.clone();
                        attrs.stretch = stretch;
                        set_text_attrs(cosmic_buffer, font_system, attrs)
                    })
            }
            if let Some(style_signal) = attrs.style {
                let style = style_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        style.signal(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), style| {
                            attrs.style = style;
                        },
                    )
                    .on_signal_with_cosmic_buffer(style.signal(), |cosmic_buffer, font_system, attrs, style| {
                        let mut attrs = attrs.0.clone();
                        attrs.style = style;
                        set_text_attrs(cosmic_buffer, font_system, attrs)
                    })
            }
            if let Some(weight_signal) = attrs.weight {
                let weight = weight_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        weight.signal(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), weight| {
                            attrs.weight = weight;
                        },
                    )
                    .on_signal_with_cosmic_buffer(weight.signal(), |cosmic_buffer, font_system, attrs, weight| {
                        let mut attrs = attrs.0.clone();
                        attrs.weight = weight;
                        set_text_attrs(cosmic_buffer, font_system, attrs)
                    })
            }
            if let Some(metadata_signal) = attrs.metadata {
                let metadata = metadata_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        metadata.signal(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), metadata| {
                            attrs.metadata = metadata;
                        },
                    )
                    .on_signal_with_cosmic_buffer(metadata.signal(), |cosmic_buffer, font_system, attrs, metadata| {
                        let mut attrs = attrs.0.clone();
                        attrs.metadata = metadata;
                        set_text_attrs(cosmic_buffer, font_system, attrs)
                    })
            }
            if let Some(cache_key_flags_signal) = attrs.cache_key_flags {
                let cache_key_flags = cache_key_flags_signal.broadcast();
                self = self
                    .on_signal_with_default_attrs(
                        cache_key_flags.signal(),
                        move |bevy_cosmic_edit::DefaultAttrs(attrs), cache_key_flags| {
                            attrs.cache_key_flags = cache_key_flags;
                        },
                    )
                    .on_signal_with_cosmic_buffer(
                        cache_key_flags.signal(),
                        |cosmic_buffer, font_system, attrs, cache_key_flags| {
                            let mut attrs = attrs.0.clone();
                            attrs.cache_key_flags = cache_key_flags;
                            set_text_attrs(cosmic_buffer, font_system, attrs)
                        },
                    )
            }
        }
        self
    }

    pub fn cosmic_edit_unit_component<C: Component + Default>(mut self, option: impl Into<Option<bool>>) -> Self {
        if Into::<Option<bool>>::into(option).unwrap_or(false) {
            self = self.cosmic_edit_insert(C::default());
        }
        self
    }

    pub fn cosmic_edit_unit_component_signal<C: Component + Default, S: Signal<Item = bool> + Send + 'static>(
        mut self,
        component_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(component_option_signal) = component_option_signal_option.into() {
            self = self.cosmic_edit_component_signal::<C, _>(component_option_signal.map_true(C::default));
        }
        self
    }

    pub fn read_only_option(self, read_only_option: impl Into<Option<bool>>) -> Self {
        self.cosmic_edit_unit_component::<bevy_cosmic_edit::ReadOnly>(read_only_option)
    }

    pub fn read_only(self) -> Self {
        self.read_only_option(true)
    }

    pub fn read_only_signal<S: Signal<Item = bool> + Send + 'static>(
        self,
        read_only_signal_option: impl Into<Option<S>>,
    ) -> Self {
        self.cosmic_edit_unit_component_signal::<bevy_cosmic_edit::ReadOnly, _>(read_only_signal_option)
    }

    pub fn scroll_disabled_option(self, scroll_disabled_option: impl Into<Option<bool>>) -> Self {
        self.cosmic_edit_unit_component::<bevy_cosmic_edit::ScrollDisabled>(scroll_disabled_option)
    }

    pub fn scroll_disabled(self) -> Self {
        self.scroll_disabled_option(true)
    }

    pub fn scroll_disabled_signal<S: Signal<Item = bool> + Send + 'static>(
        self,
        scroll_disabled_signal_option: impl Into<Option<S>>,
    ) -> Self {
        self.cosmic_edit_unit_component_signal::<bevy_cosmic_edit::ScrollDisabled, _>(scroll_disabled_signal_option)
    }

    pub fn user_select_none_option(self, user_select_none_option: impl Into<Option<bool>>) -> Self {
        self.cosmic_edit_unit_component::<bevy_cosmic_edit::UserSelectNone>(user_select_none_option)
    }

    pub fn user_select_none(self) -> Self {
        self.user_select_none_option(true)
    }

    pub fn user_select_none_signal<S: Signal<Item = bool> + Send + 'static>(
        self,
        user_select_none_signal_option: impl Into<Option<S>>,
    ) -> Self {
        self.cosmic_edit_unit_component_signal::<bevy_cosmic_edit::UserSelectNone, _>(user_select_none_signal_option)
    }

    pub fn placeholder(mut self, placeholder_option: impl Into<Option<PlaceHolder>>) -> Self {
        if let Some(placeholder) = Into::<Option<PlaceHolder>>::into(placeholder_option) {
            if let Some(text_signal) = placeholder.text {
                self = self.on_signal_with_cosmic_edit(text_signal, move |mut cosmic_edit, text| {
                    if let Some(mut placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                        placeholder.text = text;
                    } else {
                        cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new(text, bevy_cosmic_edit::Attrs::new()));
                    }
                });
            }
            if let Some(attrs) = placeholder.attrs {
                if let Some(color_signal) = attrs.color_opt {
                    self = self.on_signal_with_cosmic_edit(color_signal, move |mut cosmic_edit, color_option| {
                        if let Some(mut placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.color_opt = color_option;
                        } else {
                            let mut attrs = bevy_cosmic_edit::Attrs::new();
                            attrs.color_opt = color_option;
                            cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    });
                }
                if let Some(family_signal) = attrs.family_owned {
                    self = self.on_signal_with_cosmic_edit(family_signal, move |mut cosmic_edit, family| {
                        if let Some(placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.family(family.as_family());
                        } else {
                            let attrs = bevy_cosmic_edit::Attrs::new();
                            attrs.family(family.as_family());
                            cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    });
                }
                if let Some(stretch_signal) = attrs.stretch {
                    self = self.on_signal_with_cosmic_edit(stretch_signal, move |mut cosmic_edit, stretch| {
                        if let Some(placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.stretch(stretch);
                        } else {
                            let attrs = bevy_cosmic_edit::Attrs::new();
                            attrs.stretch(stretch);
                            cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    });
                }
                if let Some(style_signal) = attrs.style {
                    self = self.on_signal_with_cosmic_edit(style_signal, move |mut cosmic_edit, style| {
                        if let Some(placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.style(style);
                        } else {
                            let attrs = bevy_cosmic_edit::Attrs::new();
                            attrs.style(style);
                            cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    });
                }
                if let Some(weight_signal) = attrs.weight {
                    self = self.on_signal_with_cosmic_edit(weight_signal, move |mut cosmic_edit, weight| {
                        if let Some(placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.weight(weight);
                        } else {
                            let attrs = bevy_cosmic_edit::Attrs::new();
                            attrs.weight(weight);
                            cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    });
                }
                if let Some(metadata_signal) = attrs.metadata {
                    self = self.on_signal_with_cosmic_edit(metadata_signal, move |mut cosmic_edit, metadata| {
                        if let Some(placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                            placeholder.attrs.metadata(metadata);
                        } else {
                            let attrs = bevy_cosmic_edit::Attrs::new();
                            attrs.metadata(metadata);
                            cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                        }
                    });
                }
                if let Some(cache_key_flags_signal) = attrs.cache_key_flags {
                    self = self.on_signal_with_cosmic_edit(
                        cache_key_flags_signal,
                        move |mut cosmic_edit, cache_key_flags| {
                            if let Some(placeholder) = cosmic_edit.get_mut::<bevy_cosmic_edit::Placeholder>() {
                                placeholder.attrs.cache_key_flags(cache_key_flags);
                            } else {
                                let attrs = bevy_cosmic_edit::Attrs::new();
                                attrs.cache_key_flags(cache_key_flags);
                                cosmic_edit.insert(bevy_cosmic_edit::Placeholder::new("", attrs));
                            }
                        },
                    );
                }
            }
        }
        self
    }

    pub fn on_change(self, handler: impl FnMut(String) + Send + Sync + 'static) -> Self {
        self.with_cosmic_edit(|mut entity| {
            entity.insert(TextInputOnChange(Box::new(handler)));
        })
    }

    pub fn on_change_sync(self, string: Mutable<String>) -> Self {
        self.on_change(move |text| string.set_neq(text))
    }
}

fn cosmic_edit_entity_forwarder(entity: &mut EntityWorldMut) -> Option<Entity> {
    entity.get::<CosmicSource>().map(|cosmic_source| cosmic_source.0)
}

fn set_text_attrs(cosmic_buffer: &mut CosmicBuffer, font_system: &mut FontSystem, attrs: bevy_cosmic_edit::AttrsOwned) {
    let spans = cosmic_buffer.get_text_spans(attrs.clone());
    if let Some(list_spans) = spans.first() {
        if let Some((text, _)) = list_spans.first() {
            cosmic_buffer.set_text(font_system, text, attrs.as_attrs());
        }
    }
}

#[derive(Component)]
struct TextInputOnChange(Box<dyn FnMut(String) + Send + Sync + 'static>);

fn on_change(mut on_change_query: Query<&mut TextInputOnChange>, mut changed_events: EventReader<CosmicTextChanged>) {
    for CosmicTextChanged((entity, text)) in changed_events.read() {
        if let Ok(mut on_change) = on_change_query.get_mut(*entity) {
            (on_change.0)(text.to_string());
        }
    }
}

#[derive(Component)]
struct Focusable {
    system: SystemId<(bool,)>,
    is_focused: bool,
}

// sync focus state
fn on_focus_changed(
    focused_widget: Res<CosmicFocusedWidget>,
    mut query: Query<(&mut Focusable, &CosmicSource)>,
    mut commands: Commands,
) {
    for (mut focusable, &CosmicSource(cosmic_edit)) in query.iter_mut() {
        if Some(cosmic_edit) == focused_widget.0 {
            // TODO: remove condition when https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
            if focusable.is_focused.not() {
                focusable.is_focused = true;
                commands.run_system_with_input(focusable.system, (true,));
            }
        } else if focusable.is_focused {
            focusable.is_focused = false;
            commands.run_system_with_input(focusable.system, (false,));
        }
    }
}

pub type BoxSignalSync<'a, T> = Pin<Box<dyn Signal<Item = T> + Send + Sync + 'a>>;

pub struct TextAttrs {
    color_opt: Option<BoxSignalSync<'static, Option<CosmicColor>>>,
    family_owned: Option<BoxSignalSync<'static, bevy_cosmic_edit::FamilyOwned>>,
    stretch: Option<BoxSignalSync<'static, bevy_cosmic_edit::Stretch>>,
    style: Option<BoxSignalSync<'static, bevy_cosmic_edit::FontStyle>>,
    weight: Option<BoxSignalSync<'static, bevy_cosmic_edit::FontWeight>>,
    metadata: Option<BoxSignalSync<'static, usize>>,
    cache_key_flags: Option<BoxSignalSync<'static, bevy_cosmic_edit::CacheKeyFlags>>,
}

impl TextAttrs {
    pub fn new() -> Self {
        Self {
            color_opt: None,
            family_owned: None,
            stretch: None,
            style: None,
            weight: None,
            metadata: None,
            cache_key_flags: None,
        }
    }

    pub fn color_signal<S: Signal<Item = Option<Color>> + Send + Sync + 'static>(
        mut self,
        color_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(color_signal) = color_signal_option.into() {
            self.color_opt = Some(
                color_signal
                    .map(|color_option| color_option.map(ColorExtras::to_cosmic))
                    .apply(Box::pin),
            );
        }
        self
    }

    pub fn color(mut self, color_option: impl Into<Option<Color>>) -> Self {
        if let Some(color) = color_option.into() {
            self = self.color_signal(always(Some(color)));
        }
        self
    }

    pub fn family_signal<S: Signal<Item = bevy_cosmic_edit::FamilyOwned> + Send + Sync + 'static>(
        mut self,
        family_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(family_signal) = family_signal_option.into() {
            self.family_owned = Some(Box::pin(family_signal));
        }
        self
    }

    pub fn family(mut self, family_option: impl Into<Option<bevy_cosmic_edit::FamilyOwned>>) -> Self {
        if let Some(family) = family_option.into() {
            self = self.family_signal(always(family));
        }
        self
    }

    pub fn stretch_signal<S: Signal<Item = bevy_cosmic_edit::Stretch> + Send + Sync + 'static>(
        mut self,
        stretch_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(stretch_signal) = stretch_signal_option.into() {
            self.stretch = Some(Box::pin(stretch_signal));
        }
        self
    }

    pub fn stretch(mut self, stretch_option: impl Into<Option<bevy_cosmic_edit::Stretch>>) -> Self {
        if let Some(stretch) = stretch_option.into() {
            self = self.stretch_signal(always(stretch));
        }
        self
    }

    pub fn style_signal<S: Signal<Item = bevy_cosmic_edit::FontStyle> + Send + Sync + 'static>(
        mut self,
        style_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(style_signal) = style_signal_option.into() {
            self.style = Some(Box::pin(style_signal));
        }
        self
    }

    pub fn style(mut self, style_option: impl Into<Option<bevy_cosmic_edit::FontStyle>>) -> Self {
        if let Some(style) = style_option.into() {
            self = self.style_signal(always(style));
        }
        self
    }

    pub fn weight_signal<S: Signal<Item = bevy_cosmic_edit::FontWeight> + Send + Sync + 'static>(
        mut self,
        weight_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(weight_signal) = weight_signal_option.into() {
            self.weight = Some(Box::pin(weight_signal));
        }
        self
    }

    pub fn weight(mut self, weight_option: impl Into<Option<bevy_cosmic_edit::FontWeight>>) -> Self {
        if let Some(weight) = weight_option.into() {
            self = self.weight_signal(always(weight));
        }
        self
    }

    pub fn metadata_signal<S: Signal<Item = usize> + Send + Sync + 'static>(
        mut self,
        metadata_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(metadata_signal) = metadata_signal_option.into() {
            self.metadata = Some(Box::pin(metadata_signal));
        }
        self
    }

    pub fn metadata(mut self, metadata_option: impl Into<Option<usize>>) -> Self {
        if let Some(metadata) = metadata_option.into() {
            self = self.metadata_signal(always(metadata));
        }
        self
    }

    pub fn cache_key_flags_signal<S: Signal<Item = bevy_cosmic_edit::CacheKeyFlags> + Send + Sync + 'static>(
        mut self,
        cache_key_flags_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cache_key_flags_signal) = cache_key_flags_signal_option.into() {
            self.cache_key_flags = Some(Box::pin(cache_key_flags_signal));
        }
        self
    }

    pub fn cache_key_flags(
        mut self,
        cache_key_flags_option: impl Into<Option<bevy_cosmic_edit::CacheKeyFlags>>,
    ) -> Self {
        if let Some(cache_key_flags) = cache_key_flags_option.into() {
            self = self.cache_key_flags_signal(always(cache_key_flags));
        }
        self
    }
}

pub struct PlaceHolder {
    text: Option<BoxSignal<'static, &'static str>>,
    attrs: Option<TextAttrs>,
}

impl PlaceHolder {
    pub fn new() -> Self {
        Self {
            text: None,
            attrs: None,
        }
    }

    pub fn text_signal<S: Signal<Item = &'static str> + Send + 'static>(
        mut self,
        text_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(text_signal) = text_signal_option.into() {
            self.text = Some(Box::pin(text_signal));
        }
        self
    }

    pub fn text(mut self, text_option: impl Into<Option<&'static str>>) -> Self {
        if let Some(text) = text_option.into() {
            self = self.text_signal(always(text));
        }
        self
    }

    pub fn attrs(mut self, attrs_option: impl Into<Option<TextAttrs>>) -> Self {
        self.attrs = attrs_option.into();
        self
    }
}

macro_rules! impl_text_input_cosmic_edit_methods {
    ($($field:ident: $field_type:ty),+ $(,)?) => {
        paste::paste! {
            impl TextInput {
                $(
                    pub fn $field(mut self, [<$field _option>]: impl Into<Option<$field_type>>) -> Self {
                        if let Some($field) = [<$field _option>].into() {
                            self = self.cosmic_edit_insert($field);
                        }
                        self
                    }

                    pub fn [<with_ $field>](self, f: impl FnOnce(&mut $field_type) + Send + 'static) -> Self {
                        self.with_cosmic_edit_component(f)
                    }

                    pub fn [<$field _signal>]<S: Signal<Item = $field_type> + Send + 'static>(
                        self,
                        [<$field _signal_option>]: impl Into<Option<S>>,
                    ) -> Self {
                        self.cosmic_edit_component_signal([<$field _signal_option>])
                    }

                    pub fn [<on_signal_with_ $field>]<T: Send + 'static>(
                        self,
                        signal: impl Signal<Item = T> + Send + 'static,
                        f: impl FnMut(&mut $field_type, T) + Send + 'static,
                    ) -> Self {
                        self.on_signal_with_cosmic_edit_component(signal, f)
                    }
                )*
            }
        }
    };
}

impl_text_input_cosmic_edit_methods! {
    buffer: CosmicBuffer,
    fill_color: bevy_cosmic_edit::CosmicBackgroundColor,
    cursor_color: bevy_cosmic_edit::CursorColor,
    selection_color: bevy_cosmic_edit::SelectionColor,
    default_attrs: bevy_cosmic_edit::DefaultAttrs,
    background_image: bevy_cosmic_edit::CosmicBackgroundImage,
    max_lines: bevy_cosmic_edit::MaxLines,
    max_chars: bevy_cosmic_edit::MaxChars,
    x_offset: bevy_cosmic_edit::XOffset,
    mode: bevy_cosmic_edit::CosmicWrap,
    text_position: bevy_cosmic_edit::CosmicTextAlign,
    padding: bevy_cosmic_edit::CosmicPadding,
    widget_size: bevy_cosmic_edit::CosmicWidgetSize,
    hover_cursor: bevy_cosmic_edit::HoverCursor,
}

pub(crate) struct TextInputPlugin;
impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_cosmic_edit::CosmicEditPlugin::default())
            .add_systems(
                Update,
                (
                    on_change.run_if(any_with_component::<TextInputOnChange>.and_then(on_event::<CosmicTextChanged>())),
                    on_focus_changed.run_if(resource_changed::<CosmicFocusedWidget>),
                )
                    .run_if(any_with_component::<CosmicSource>),
            )
            .add_systems(PostUpdate, bevy_cosmic_edit::deselect_editor_on_esc);
    }
}
