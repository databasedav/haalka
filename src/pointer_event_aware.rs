//! Semantics for managing how an [`Element`](super::element::Element) reacts to pointer events like
//! hover, click, and press, see [`PointerEventAware`].

use std::{
    ops::Not,
    sync::{Arc, OnceLock},
    time::Duration,
};

use apply::Apply;
use bevy_app::prelude::*;
use bevy_derive::*;
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_log::prelude::*;
use bevy_picking::{
    backend::prelude::*,
    hover::{HoverMap, PickingInteraction, PreviousHoverMap},
    pointer::PointerMap,
    prelude::*,
};
use bevy_reflect::prelude::*;
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::Pressed;
use bevy_window::*;
use jonmo::signal::{Signal, SignalExt};

use super::{
    element::UiRoot,
    global_event_aware::GlobalEventAware,
    utils::{HaalkaObserver, clone, observe, register_system, remove_system_holder_on_remove},
};

/// Helper to create a signal that always outputs a constant value.
fn constant_signal<T: Clone + Send + Sync + 'static>(value: T) -> impl Signal<Item = T> + Send + Sync + 'static {
    SignalBuilder::from_system(move |_: In<()>| Some(value.clone()))
}

use jonmo::signal::SignalBuilder;

/// Handler data for hover events, containing hover state and hit information.
#[derive(Clone)]
pub struct HoverData {
    /// Whether the element is currently hovered.
    pub hovered: bool,
    /// Hit information for the pointer intersection.
    pub hit: HitData,
}

/// Handler data for press events, containing press state, button, and hit information.
#[derive(Clone)]
pub struct PressData {
    /// Whether the element is currently pressed.
    pub pressed: bool,
    /// The button that was pressed.
    pub button: PointerButton,
    /// Hit information for the pointer intersection.
    pub hit: HitData,
}

/// Enables reacting to pointer events like hover, click, and press. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`PointerEventAware`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/ability/pointer_event_aware.rs).
pub trait PointerEventAware: GlobalEventAware {
    /// On frames where this element is hovered or gets unhovered and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and [`HoverHandlerData`] containing its current hovered state and the latest [`HitData`].
    /// While hovered, the handler will be executed every frame.
    fn on_hovered_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let hover_handler_holder = Arc::new(OnceLock::new());
            let hovering_handler_holder = Arc::new(OnceLock::new());
            builder
                .on_spawn(clone!((hover_handler_holder, hovering_handler_holder) move |world, entity| {
                    let hover_handler_system = register_system(world, handler);
                    let _ = hover_handler_holder.set(hover_handler_system);

                    let hovering_handler_system = register_system(
                        world,
                        move |In(entity): In<Entity>,
                              hover_datas: Query<&HoverDataInternal>,
                              blocked: Query<&Blocked>,
                              mut commands: Commands| {
                            if blocked.contains(entity) {
                                return;
                            }
                            if let Ok(hover_data) = hover_datas.get(entity) {
                                commands.run_system_with(hover_handler_system, (entity, HoverData {
                                    hovered: true,
                                    hit: hover_data.hit.clone(),
                                }));
                            }
                        },
                    );
                    let _ = hovering_handler_holder.set(hovering_handler_system);

                    observe(
                        world,
                        entity,
                        move |mut enter: On<Pointer<Enter>>,
                              blocked: Query<&Blocked>,
                              move_observers: Query<&HoverMoveObserver>,
                              mut commands: Commands| {
                            enter.propagate(false);

                            let entity = enter.entity;
                            if blocked.contains(entity) {
                                return;
                            }

                            let hit = enter.hit.clone();

                            let move_observer = (!move_observers.contains(entity)).then(|| {
                                commands
                                    .spawn((
                                        Observer::new(
                                            |move_event: On<Pointer<Move>>, mut hover_datas: Query<&mut HoverDataInternal>| {
                                                let entity = move_event.entity;
                                                if let Ok(mut hover_data) = hover_datas.get_mut(entity) {
                                                    hover_data.hit = move_event.hit.clone();
                                                }
                                            },
                                        )
                                        .with_entity(entity),
                                        HaalkaObserver,
                                    ))
                                    .id()
                            });

                            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                                entity_commands.insert(HoverDataInternal { hit: hit.clone() });
                                entity_commands.insert(Hovering);
                                entity_commands.insert(HoveredSystem(hovering_handler_system));
                                if let Some(move_observer) = move_observer {
                                    entity_commands.insert(HoverMoveObserver(move_observer));
                                }
                            }

                            commands.run_system_with(hover_handler_system, (entity, HoverData {
                                hovered: true,
                                hit,
                            }));
                        },
                    );

                    observe(
                        world,
                        entity,
                        move |mut leave: On<Pointer<Leave>>,
                              blocked: Query<&Blocked>,
                              move_observers: Query<&HoverMoveObserver>,
                              mut commands: Commands| {
                            leave.propagate(false);
                            let entity = leave.entity;

                            let hit = leave.hit.clone();

                            if !blocked.contains(entity) {
                                commands.run_system_with(hover_handler_system, (entity, HoverData {
                                    hovered: false,
                                    hit,
                                }));
                            }

                            let move_observer = move_observers.get(entity).ok().map(|o| o.0);

                            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                                entity_commands.remove::<HoverDataInternal>();
                                entity_commands.remove::<Hovering>();
                                entity_commands.remove::<HoveredSystem>();
                                if move_observer.is_some() {
                                    entity_commands.remove::<HoverMoveObserver>();
                                }
                            }

                            if let Some(observer) = move_observer {
                                commands.entity(observer).despawn();
                            }
                        },
                    );
                }))
                .apply(remove_system_holder_on_remove(hover_handler_holder))
                .apply(remove_system_holder_on_remove(hovering_handler_holder))
        })
    }

    /// Like [`PointerEventAware::on_hovered_blockable`], but reactively controls whether hover
    /// handling is blocked with a [`Signal`].
    fn on_hovered_blockable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<HoverHandlingBlocked, _>(
                blocked.map_true(|_: In<()>| HoverHandlingBlocked::default()),
            )
        })
        .on_hovered_blockable::<HoverHandlingBlocked, _>(handler)
    }

    /// When this element's hovered state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HoverHandlerData`]. This method
    /// can be called repeatedly to register many such handlers.
    fn on_hovered_change<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(clone!((system_holder) move |builder| {
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        }))
        .on_hovered_blockable::<HoverHandlingBlocked, _>(
            move |In((entity, data)): In<(Entity, HoverData)>,
                  mut prev: Local<Option<bool>>,
                  mut commands: Commands| {
                if prev.map_or(true, |prev| prev != data.hovered) {
                    *prev = Some(data.hovered);
                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data));
                }
            },
        )
    }

    /// On frames where this element is hovered and does not have a `Blocked` [`Component`], run a
    /// [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`HitData`].
    /// This method can be called repeatedly to register many such handlers.
    fn on_hovering_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HitData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(clone!((system_holder) move |builder| {
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        }))
        .on_hovered_blockable::<Blocked, _>(
            move |In((entity, data)): In<(Entity, HoverData)>, mut commands: Commands| {
                if data.hovered {
                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data.hit));
                }
            },
        )
    }

    /// Like [`PointerEventAware::on_hovering_blockable`], but reactively controls whether hover
    /// handling is blocked with a [`Signal`].
    fn on_hovering_blockable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HitData)>, (), Marker> + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<HoverHandlingBlocked, _>(
                blocked.map_true(|_: In<()>| HoverHandlingBlocked::default()),
            )
        })
        .on_hovering_blockable::<HoverHandlingBlocked, _>(handler)
    }

    /// On frames where this element is hovered, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HitData`].
    fn on_hovering<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HitData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_hovering_blockable::<HoverHandlingBlocked, _>(handler)
    }

    /// On frames where this element is hovered, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HitData`], throttled by `duration`
    /// before the `handler` can run again.
    fn on_hovering_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HitData)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .insert(HoverThrottleTimer(Timer::new(duration, TimerMode::Once)))
                .on_spawn(
                    clone!((system_holder) move |world, _| { let _ = system_holder.set(register_system(world, handler)); }),
                )
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        })
        .on_hovered_blockable::<HoverHandlingBlocked, _>(
            move |In((entity, data)): In<(Entity, HoverData)>, mut commands: Commands, time: Res<Time>, mut timers: Query<&mut HoverThrottleTimer>| {
                if data.hovered {
                    if let Ok(mut timer) = timers.get_mut(entity) {
                        timer.0.tick(time.delta());
                        if timer.0.is_finished() {
                            commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data.hit));
                            timer.0.reset();
                        }
                    }
                }
            },
        )
    }

    /// Run a [`System`] when this element is clicked.
    fn on_click<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |click: On<Pointer<Click>>, mut commands: Commands| {
                        commands.run_system_with(system, (click.entity, (*click).clone()));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// Run a [`System`] when this element is clicked, reactively controlling whether the click
    /// bubbles up the hierarchy with a [`Signal`].
    fn on_click_propagation_stoppable<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
        propagation_stopped: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .component_signal::<ClickPropagationStopped, _>(
                    propagation_stopped.map_true(|_: In<()>| ClickPropagationStopped),
                )
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |mut click: On<Pointer<Click>>, propagation_stopped: Query<&ClickPropagationStopped>, mut commands: Commands| {
                        if propagation_stopped.contains(click.entity) {
                            click.propagate(false);
                        }
                        commands.run_system_with(system, (click.entity, (*click).clone()));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// Run a [`System`] when this element is left clicked, stopping the click from bubbling up the
    /// hierarchy.
    fn on_click_stop_propagation<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.insert((Pickable::default(), ClickPropagationStopped)).observe(
                move |mut click: On<Pointer<Click>>| {
                    click.propagate(false);
                },
            )
        })
        .on_click(handler)
    }

    /// Run a [`System`] when this element is right clicked.
    fn on_right_click<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |click: On<Pointer<Click>>, mut commands: Commands| {
                        if matches!(click.button, PointerButton::Secondary) {
                            commands.run_system_with(system, (click.entity, (*click).clone()));
                        }
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a [`System`] that takes [`In`](`System::In`) this element's
    /// [`Entity`] and the [`Pointer<Click>`]. Will not function unless this element is a descendant
    /// of a [`UiRoot`]. This method can be called repeatedly to register many such handlers.
    fn on_click_outside<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        })
        .on_global_event_with_system::<Pointer<Click>, _>(
            move |In((entity, click)): In<(Entity, Pointer<Click>)>,
                  children: Query<&Children>,
                  child_ofs: Query<&ChildOf>,
                  ui_roots: Query<&UiRoot>,
                  mut commands: Commands| {
                for ancestor in child_ofs.iter_ancestors(entity) {
                    if ui_roots.contains(ancestor) {
                        if !is_inside_or_removed_from_dom(entity, &click, ancestor, &children) {
                            commands.run_system_with(system_holder.get().copied().unwrap(), (entity, click));
                        }
                        break;
                    }
                }
            },
        )
    }

    /// On frames where this element is pressed or gets unpressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and [`PressHandlerData`]. This method can be called repeatedly to register many such
    /// handlers.
    fn on_pressed_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let press_handler_holder = Arc::new(OnceLock::new());
            let pressing_handler_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pickable::default())
                .on_spawn(clone!((press_handler_holder, pressing_handler_holder) move |world, entity| {
                    let press_handler_system = register_system(world, handler);
                    let _ = press_handler_holder.set(press_handler_system);

                    let pressing_handler_system = register_system(
                        world,
                        move |In(entity): In<Entity>,
                              press_datas: Query<&PressDataInternal>,
                              blocked: Query<&Blocked>,
                              mut commands: Commands| {
                            if blocked.contains(entity) {
                                return;
                            }
                            if let Ok(press_data) = press_datas.get(entity) {
                                commands.run_system_with(
                                    press_handler_system,
                                    (entity, PressData {
                                        pressed: true,
                                        button: press_data.button,
                                        hit: press_data.hit.clone(),
                                    }),
                                );
                            }
                        },
                    );
                    let _ = pressing_handler_holder.set(pressing_handler_system);

                    observe(
                        world,
                        entity,
                        move |press: On<Pointer<Press>>,
                              blocked: Query<&Blocked>,
                              move_observers: Query<&PressMoveObserver>,
                              mut commands: Commands| {
                            let entity = press.entity;
                            if blocked.contains(entity) {
                                return;
                            }

                            let button = press.button;
                            let hit = press.hit.clone();

                            let move_observer = (!move_observers.contains(entity)).then(|| {
                                commands
                                    .spawn((
                                        Observer::new(
                                            |move_event: On<Pointer<Move>>, mut press_datas: Query<&mut PressDataInternal>| {
                                                let entity = move_event.entity;
                                                if let Ok(mut press_data) = press_datas.get_mut(entity) {
                                                    press_data.hit = move_event.hit.clone();
                                                }
                                            },
                                        )
                                        .with_entity(entity),
                                        HaalkaObserver,
                                    ))
                                    .id()
                            });

                            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                                entity_commands.insert(PressDataInternal {
                                    button,
                                    hit: hit.clone(),
                                });
                                entity_commands.insert(PressedSystem(pressing_handler_system));
                                if let Some(move_observer) = move_observer {
                                    entity_commands.insert(PressMoveObserver(move_observer));
                                }
                            }

                            commands.run_system_with(press_handler_system, (entity, PressData {
                                pressed: true,
                                button,
                                hit,
                            }));
                        },
                    );

                    observe(
                        world,
                        entity,
                        move |release: On<Pointer<Release>>,
                              blocked: Query<&Blocked>,
                              move_observers: Query<&PressMoveObserver>,
                              press_datas: Query<&PressDataInternal>,
                              mut commands: Commands| {
                            let entity = release.entity;

                            if let Ok(press_data) = press_datas.get(entity) {
                                if press_data.button != release.button {
                                    return;
                                }
                            } else {
                                return;
                            }

                            let button = release.button;
                            let hit = release.hit.clone();

                            let move_observer = move_observers.get(entity).ok().map(|o| o.0);

                            if !blocked.contains(entity) {
                                commands.run_system_with(press_handler_system, (entity, PressData {
                                    pressed: false,
                                    button,
                                    hit,
                                }));
                            }

                            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                                entity_commands.remove::<PressDataInternal>();
                                entity_commands.remove::<PressedSystem>();
                                if move_observer.is_some() {
                                    entity_commands.remove::<PressMoveObserver>();
                                }
                            }

                            if let Some(observer) = move_observer {
                                commands.entity(observer).despawn();
                            }
                        },
                    );
                }))
                .apply(remove_system_holder_on_remove(press_handler_holder))
                .apply(remove_system_holder_on_remove(pressing_handler_holder))
        })
        .on_hovered_change(
            |In((entity, data)): In<(Entity, HoverData)>,
             move_observers: Query<&PressMoveObserver>,
             mut commands: Commands| {
                if !data.hovered {
                    if let Ok(&PressMoveObserver(observer)) = move_observers.get(entity) {
                        commands.entity(observer).despawn();
                    }
                    if let Ok(mut entity_commands) = commands.get_entity(entity) {
                        entity_commands.remove::<PressMoveObserver>();
                        entity_commands.remove::<PressDataInternal>();
                        entity_commands.remove::<PressedSystem>();
                    }
                }
            },
        )
    }

    /// Like [`PointerEventAware::on_pressed_blockable`], but reactively controls whether press
    /// handling is blocked with a [`Signal`].
    fn on_pressed_blockable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<PressHandlingBlocked, _>(
                blocked.map_true(|_: In<()>| PressHandlingBlocked::default()),
            )
        })
        .on_pressed_blockable::<PressHandlingBlocked, _>(handler)
    }

    /// When this element's pressed state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`PressHandlerData`]. This method
    /// can be called repeatedly to register many such handlers.
    fn on_pressed_change<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(clone!(
            (system_holder) | builder | {
                builder.on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
            }
        ))
        .with_builder(remove_system_holder_on_remove(system_holder.clone()))
        .on_pressed_blockable::<PressHandlingBlocked, _>(
            move |In((entity, data)): In<(Entity, PressData)>,
                  mut pressed: Local<Option<bool>>,
                  mut commands: Commands| {
                if pressed.is_none_or(|prev| prev != data.pressed) {
                    *pressed = Some(data.pressed);
                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data));
                }
            },
        )
    }

    /// On frames where this element is being pressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`], [`PointerButton`], and [`HitData`]. This method can be called repeatedly
    /// to register many such handlers.
    fn on_pressing_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PointerButton, HitData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(clone!(
            (system_holder) | builder | {
                builder
                    .on_spawn(clone!((system_holder) move |world, _| {
                        let _ = system_holder.set(register_system(world, handler));
                    }))
                    .apply(remove_system_holder_on_remove(system_holder.clone()))
            }
        ))
        .on_pressed_blockable::<Blocked, _>(
            move |In((entity, data)): In<(Entity, PressData)>, mut commands: Commands| {
                if data.pressed {
                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data.button, data.hit));
                }
            },
        )
    }

    /// On frames where this element is being pressed, run a [`System`], reactively controlling
    /// whether the press is blocked with a [`Signal`].
    fn on_pressing_blockable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PointerButton, HitData)>, (), Marker> + Send + Sync + 'static,
        blocked: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<PressHandlingBlocked, _>(
                blocked.map_true(|_: In<()>| PressHandlingBlocked::default()),
            )
        })
        .on_pressing_blockable::<PressHandlingBlocked, _>(handler)
    }

    /// When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`], [`PointerButton`], and [`HitData`].
    fn on_pressing<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PointerButton, HitData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_pressing_blockable::<PressHandlingBlocked, _>(handler)
    }

    /// When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`], [`PointerButton`], and [`HitData`], throttled by `duration` before
    /// the `handler` can run again.
    fn on_pressing_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PointerButton, HitData)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .insert(PressThrottleTimer(Timer::new(duration, TimerMode::Once)))
                .on_spawn(
                    clone!((system_holder) move |world, _| { let _ = system_holder.set(register_system(world, handler)); }),
                )
                .apply(remove_system_holder_on_remove(system_holder.clone()))
            })
        .on_pressed_blockable::<PressHandlingBlocked, _>(
            move |In((entity, data)): In<(Entity, PressData)>, mut commands: Commands, time: Res<Time>, mut timers: Query<&mut PressThrottleTimer>| {
                if data.pressed {
                    if let Ok(mut timer) = timers.get_mut(entity) {
                        timer.0.tick(time.delta());
                        if timer.0.is_finished() {
                            commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data.button, data.hit));
                            timer.0.reset();
                        }
                    }
                }
            },
        )
    }
}

/// Timer component for throttling press events.
#[derive(Component, Deref, DerefMut)]
struct PressThrottleTimer(Timer);

/// Timer component for throttling hover events.
#[derive(Component, Deref, DerefMut)]
struct HoverThrottleTimer(Timer);

#[derive(Component, Clone)]
pub struct Hovered;

#[derive(Component, Default, Clone)]
struct PressHandlingBlocked;

#[derive(Component, Default, Clone)]
struct HoverHandlingBlocked;

/// Fires when a the pointer crosses into the bounds of the `target` entity, ignoring children.
#[derive(Clone, PartialEq, Debug, Reflect, Event)]
pub struct Enter {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a the pointer crosses out of the bounds of the `target` entity, excluding children.
#[derive(Clone, PartialEq, Debug, Reflect, Event)]
pub struct Leave {
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
}

// TODO: integrate with bubbling observers and upstreamed event listener
fn update_hover_states(
    pointer_map: Res<PointerMap>,
    pointers: Query<&PointerLocation>,
    hover_map: Res<HoverMap>,
    previous_hover_map: Res<PreviousHoverMap>,
    mut hovereds: Query<(Entity, Option<&Hovered>)>,
    child_ofs: Query<&ChildOf>,
    mut commands: Commands,
) {
    let pointer_id = PointerId::Mouse;
    let hover_set = hover_map.get(&pointer_id);
    for (entity, hovered) in hovereds.iter_mut() {
        let hit_data_option = match hover_set {
            Some(map) => map
                .iter()
                .find(|(ha, _)| **ha == entity || child_ofs.iter_ancestors(**ha).any(|e| e == entity))
                .map(|(_, hit_data)| hit_data),
            None => None,
        };
        let is_hovered = hit_data_option.is_some();
        if hovered.is_some() != is_hovered {
            let Some(location) = pointer_map
                .get_entity(pointer_id)
                .and_then(|entity| pointers.get(entity).ok())
                .and_then(|pointer| pointer.location.clone())
            else {
                debug!(
                    "Unable to get location for pointer {:?} during pointer {}",
                    pointer_id,
                    if is_hovered { "enter" } else { "leave" }
                );
                continue;
            };
            if let Some(hit) = hit_data_option.cloned() {
                commands.trigger(Pointer::new(pointer_id, location, Enter { hit }, entity));
                if let Ok(mut entity) = commands.get_entity(entity) {
                    entity.insert(Hovered);
                }
            } else {
                let previous_hit = previous_hover_map.get(&pointer_id).and_then(|map| {
                    map.iter()
                        .find(|(ha, _)| **ha == entity || child_ofs.iter_ancestors(**ha).any(|e| e == entity))
                        .map(|(_, hit)| hit.clone())
                });

                if let Some(hit) = previous_hit {
                    commands.trigger(Pointer::new(pointer_id, location, Leave { hit }, entity));
                    if let Ok(mut entity) = commands.get_entity(entity) {
                        entity.remove::<Hovered>();
                    }
                } else {
                    debug!(
                        "Unable to get previous hit for pointer {:?} leave on {:?}",
                        pointer_id, entity
                    );
                }
            }
        }
    }
}

#[derive(Component, Clone)]
struct ClickPropagationStopped;

#[derive(Component)]
struct OutPropagationStopped;

#[derive(Component, Clone)]
struct PressDataInternal {
    button: PointerButton,
    hit: HitData,
}

#[derive(Component, Clone, Copy)]
struct PressMoveObserver(Entity);

#[derive(Component, Clone)]
struct HoverDataInternal {
    hit: HitData,
}

#[derive(Component, Clone, Copy)]
struct HoverMoveObserver(Entity);

#[derive(Component, Clone, Copy)]
struct HoveredSystem(SystemId<In<Entity>, ()>);

#[derive(Component)]
pub(crate) struct Hoverable;

#[derive(Component, Clone, Copy)]
struct Hovering;

#[derive(Component)]
pub(crate) struct Pressable;

#[derive(Component)]
struct PressedSystem(SystemId<In<Entity>, ()>);

#[allow(clippy::type_complexity)]
fn pressed_system(mut interaction_query: Query<(Entity, &PressedSystem), With<Pressed>>, mut commands: Commands) {
    for (entity, &PressedSystem(system)) in &mut interaction_query {
        commands.run_system_with(system, entity);
    }
}

#[allow(clippy::type_complexity)]
fn pressable_system(
    mut interaction_query: Query<
        (Entity, &PickingInteraction, Option<&Pressed>),
        (With<Pressable>, Changed<PickingInteraction>),
    >,
    mut commands: Commands,
) {
    for (entity, interaction, pressed_option) in &mut interaction_query {
        let is_pressed = matches!(interaction, PickingInteraction::Pressed);
        if is_pressed != pressed_option.is_some()
            && let Ok(mut entity) = commands.get_entity(entity)
        {
            if is_pressed {
                entity.insert(Pressed);
            } else {
                entity.remove::<Pressed>();
            }
        }
    }
}

fn hoverable_system(mut hovering_query: Query<(Entity, &HoveredSystem), With<Hovering>>, mut commands: Commands) {
    for (entity, &HoveredSystem(system)) in &mut hovering_query {
        commands.run_system_with(system, entity);
    }
}

fn contains(left: Entity, right: Entity, children_query: &Query<&Children>) -> bool {
    left == right || children_query.iter_descendants(left).any(|e| e == right)
}

// TODO: add support for some sort of exclusion
// ported from moonzoon https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/ability/mouse_event_aware.rs#L158
fn is_inside_or_removed_from_dom(
    element: Entity,
    event: &Pointer<Click>,
    ui_root: Entity,
    children_query: &Query<&Children>,
) -> bool {
    if contains(element, event.entity, children_query) {
        return true;
    }
    if !contains(ui_root, event.entity, children_query) {
        return true;
    }
    false
}

#[derive(Component)]
struct CursorOver;

#[derive(Component, Default, Clone)]
struct CursorDisabled;

/// When this [`Resource`] exists in the [`World`], [`CursorOnHoverable`]
/// [`Element`]s will not trigger updates to the window's cursor when they
/// receive a [`Pointer<Over>`] event. When this [`Resource`] is removed, the last
/// [`Option<CursorIcon>`] queued by a [`CursorOnHoverable`] [`Element`] will be set as the window's
/// cursor. Adding this [`Resource`] to the [`World`] will *not* unset any [`Option<CursorIcon>`]s
/// previously set by a [`CursorOnHoverable`] [`Element`].
///
/// [`Element`]: super::element::Element
#[derive(Resource)]
pub struct CursorOnHoverDisabled;

/// A [`Component`] which stores the [`Option<CursorIcon>`] to set the window's cursor to when an
/// [`Element`](super::element::Element) receives a [`Pointer<Over>`] event; when [`None`], the
/// cursor will be hidden.
#[derive(Component, Clone)]
pub struct CursorOnHover(Option<CursorIcon>);

/// Enables managing the window's [`CursorIcon`] when an [`Element`](super::element::Element)
/// receives an [`Pointer<Over>`] event.
pub trait CursorOnHoverable: PointerEventAware {
    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] in the [`CursorOnHover`] [`Component`] or hide it if [`None`].
    /// If the [`Pointer`] is [`Over`] this element when it is disabled with a `Disabled`
    /// [`Component`], another [`Pointer<Over>`] event will be sent up the hierarchy to trigger
    /// any handlers whose propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_disableable<Disabled: Component>(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        let cursor_option = cursor_option.into();
        self.with_builder(|builder| {
            builder
                .insert((Pickable::default(), CursorOverPropagationStopped))
                .observe(
                    |event: On<Insert, CursorOver>,
                     cursor_on_hovers: Query<&CursorOnHover>,
                     disabled: Query<&Disabled>,
                     cursor_over_disabled_option: Option<Res<CursorOnHoverDisabled>>,
                     mut commands: Commands| {
                        let entity = event.entity;
                        if let Ok(CursorOnHover(cursor_option)) = cursor_on_hovers.get(entity).cloned() {
                            if cursor_over_disabled_option.is_none() {
                                if disabled.contains(entity).not() {
                                    commands.trigger(SetCursor(cursor_option));
                                }
                            } else {
                                commands.insert_resource(QueuedCursor(cursor_option));
                            }
                        }
                    },
                )
                .observe(
                    |event: On<Insert, CursorOnHover>, cursor_overs: Query<&CursorOver>, mut commands: Commands| {
                        let entity = event.entity;
                        if cursor_overs.contains(entity)
                            && let Ok(mut entity) = commands.get_entity(entity)
                        {
                            entity.try_insert(CursorOver);
                        }
                    },
                )
                .insert(CursorOnHover(cursor_option))
                .observe(
                    move |event: On<Add, Disabled>,
                          cursor_over: Query<&CursorOver>,
                          pointer_map: Res<PointerMap>,
                          pointers: Query<&PointerLocation>,
                          hover_map: Res<HoverMap>,
                          mut pointer_over: MessageWriter<Pointer<Over>>,
                          child_ofs: Query<&ChildOf>,
                          mut commands: Commands| {
                        let entity = event.event().entity;
                        if let Ok(mut entity) = commands.get_entity(entity) {
                            entity.remove::<CursorOverPropagationStopped>();
                        }
                        if cursor_over.get(entity).is_ok()
                            && let Some(((hover_map, location), &ChildOf(parent))) = hover_map
                                .get(&PointerId::Mouse)
                                .zip(
                                    pointer_map
                                        .get_entity(PointerId::Mouse)
                                        .and_then(|entity| pointers.get(entity).ok())
                                        .and_then(|pointer| pointer.location.clone()),
                                )
                                .zip(child_ofs.get(entity).ok())
                            && let Some(hit) = hover_map.get(&entity).cloned()
                        {
                            pointer_over.write(Pointer::new(PointerId::Mouse, location, Over { hit }, parent));
                        }
                    },
                )
                .observe(
                    move |event: On<Remove, Disabled>, cursor_over: Query<&CursorOver>, mut commands: Commands| {
                        let entity = event.event().entity;
                        if let Ok(mut entity_commands) = commands.get_entity(entity) {
                            entity_commands.try_insert(CursorOverPropagationStopped);
                            if cursor_over.get(entity).is_ok() {
                                entity_commands.try_insert(CursorOver);
                            }
                        }
                    },
                )
                .observe(
                    |mut over: On<Pointer<Over>>,
                     propagation_stopped: Query<&CursorOverPropagationStopped>,
                     mut commands: Commands| {
                        let entity = over.entity;
                        if propagation_stopped.contains(entity) {
                            over.propagate(false);
                        }
                        if let Ok(mut entity) = commands.get_entity(entity) {
                            entity.try_insert(CursorOver);
                        }
                    },
                )
                .insert(OutPropagationStopped)
                .observe(|mut out: On<Pointer<Out>>, mut commands: Commands| {
                    out.propagate(false);
                    if let Ok(mut entity) = commands.get_entity(out.entity) {
                        entity.remove::<CursorOver>();
                    }
                })
        })
    }

    /// When this [`Element`](super::element::Element) receives a [`Pointer<Over>`] event, set the
    /// window's cursor to [`Some`] [`CursorIcon`] in the [`CursorOnHover`] [`Component`] or
    /// hide it if [`None`].
    fn cursor(self, cursor_option: impl Into<Option<CursorIcon>>) -> Self {
        self.cursor_disableable::<CursorDisabled>(cursor_option)
    }

    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] output by the [`Signal`] or hide it if [`None`]. If the
    /// [`Pointer`] is [`Over`] this element when it is disabled with a `Disabled`
    /// [`Component`], another [`Pointer<Over>`] event will be sent up the hierarchy to trigger
    /// any handlers whose propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_signal_disableable<Disabled: Component>(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<CursorOnHover, _>(cursor_option_signal.map_in(|x| Some(CursorOnHover(x.into()))))
        })
        .cursor_disableable::<Disabled>(None)
    }

    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] output by the [`Signal`] or hide it if [`None`]. If the
    /// [`Pointer`] is [`Over`] this element when it is disabled with the `disabled` [`Signal`]
    /// outputting `true`, another [`Pointer<Over>`] event will be sent up the hierarchy to
    /// trigger any handlers whose propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_signal_disableable_signal(
        self,
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<CursorDisabled, _>(disabled.map_true(|_: In<()>| CursorDisabled::default()))
        })
        .cursor_signal_disableable::<CursorDisabled>(cursor_option_signal)
    }

    /// When this [`Element`](super::element::Element) receives a [`Pointer<Over>`] event, set the
    /// window's cursor to [`Some`] [`CursorIcon`] output by the [`Signal`] or hide it if
    /// [`None`].
    fn cursor_signal<S: Signal<Item = impl Into<Option<CursorIcon>> + 'static> + Send + Sync + 'static>(
        mut self,
        cursor_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cursor_option_signal) = cursor_option_signal_option.into() {
            self = self.cursor_signal_disableable::<CursorDisabled>(cursor_option_signal);
        }
        self
    }

    /// When this [`Element`] receives a [`Pointer<Over>`] event, set the window's cursor to
    /// [`Some`] [`CursorIcon`] or hide it if [`None`]. If the [`Pointer`] is [`Over`] this
    /// element when it is disabled with the `disabled` [`Signal`] outputting `true`, another
    /// [`Pointer<Over>`] event will be sent up the hierarchy to trigger any handlers whose
    /// propagation was previously stopped by this [`Element`].
    ///
    /// [`Element`]: super::element::Element
    fn cursor_disableable_signal(
        self,
        cursor_option: impl Into<Option<CursorIcon>>,
        disabled: impl Signal<Item = bool> + Send + Sync + 'static,
    ) -> Self {
        self.cursor_signal_disableable_signal(constant_signal(cursor_option.into()), disabled)
    }
}

/// [`Event`] consumed by a global [`Observer`] to set the window's [`CursorIcon`]; the cursor will
/// be hidden if [`None`].
#[derive(Event)]
pub struct SetCursor(pub Option<CursorIcon>);

#[derive(Component)]
struct CursorOverPropagationStopped;

#[derive(Resource)]
struct QueuedCursor(Option<CursorIcon>);

fn consume_queued_cursor(queued_cursor: Option<Res<QueuedCursor>>, mut commands: Commands) {
    if let Some(cursor) = queued_cursor {
        commands.trigger(SetCursor(cursor.0.clone()));
        commands.remove_resource::<QueuedCursor>();
    }
}

// TODO: add support for multiple windows
fn on_set_cursor(
    event: On<SetCursor>,
    mut cursor_options: Query<(Entity, &mut CursorOptions), With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if let Ok((entity, mut cursor_options)) = cursor_options.single_mut() {
        let SetCursor(icon_option) = event.event();
        if let Some(icon) = icon_option.clone() {
            if let Ok(mut window) = commands.get_entity(entity) {
                window.try_insert(icon);
            }
            cursor_options.visible = true;
        } else {
            cursor_options.visible = false;
        }
    }
}

/// When this [`Resource`] exists in the [`World`], [`Enter`] and [`Leave`] events will not be
/// fired.
#[derive(Resource)]
pub struct UpdateHoverStatesDisabled;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(on_set_cursor).add_systems(
        Update,
        (
            (
                pressable_system.run_if(any_with_component::<Pressable>),
                pressed_system.run_if(any_with_component::<PressedSystem>),
            )
                .chain(),
            hoverable_system.run_if(any_with_component::<HoveredSystem>),
            update_hover_states.run_if(
                any_with_component::<Hoverable>
                    // TODO: apparently this updates every frame no matter what, if so, remove this condition
                    // TODO: remove when native `Enter` and `Leave` available
                    .and(resource_exists_and_changed::<HoverMap>)
                    .and(not(resource_exists::<UpdateHoverStatesDisabled>)),
            ),
            consume_queued_cursor.run_if(resource_removed::<CursorOnHoverDisabled>),
        ),
    );
}
