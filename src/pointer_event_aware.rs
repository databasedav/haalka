//! Semantics for managing how an [`Element`](super::element::Element) reacts to pointer events like
//! hover, click, and press, see [`PointerEventAware`].

use std::{
    ops::Not,
    sync::{Arc, OnceLock},
    time::Duration,
};

use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_log::prelude::*;
use bevy_math::Vec2;
use bevy_picking::{
    backend::prelude::*,
    hover::{HoverMap, PickingInteraction, PreviousHoverMap},
    pointer::{Location, PointerMap},
    prelude::*,
};
use bevy_reflect::prelude::*;
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::Pressed;
use bevy_window::*;
use jonmo::signal::{Signal, SignalExt};

use super::{
    element::UiRoot,
    global_event_aware::{GlobalEventAware, GlobalEventData},
    utils::{HaalkaObserver, clone, observe, register_system, remove_system_holder_on_remove},
};

use jonmo::signal::SignalBuilder;

/// Helper trait for internal data components that track pointer state.
trait PointerDataInternal {
    fn update_from_move(&mut self, hit: HitData, pointer_location: Location);
}

/// Macro to create a move observer for a specific data type.
macro_rules! create_move_observer {
    ($commands:expr, $entity:expr, $data_type:ty) => {{
        let entity = $entity;
        $commands
            .spawn((
                Observer::new(|move_event: On<Pointer<Move>>, mut datas: Query<&mut $data_type>| {
                    let entity = move_event.entity;
                    if let Ok(mut data) = datas.get_mut(entity) {
                        data.update_from_move(move_event.hit.clone(), move_event.pointer_location.clone());
                    }
                })
                .with_entity(entity),
                HaalkaObserver,
            ))
            .id()
    }};
}

/// Macro to generate blockable_signal methods that add signal-based blocking.
macro_rules! impl_blockable_signal {
    ($(#[$attr:meta])* $method_name:ident, $blockable_method:ident, $blocked_component:ty, $data_type:ty) => {
        $(#[$attr])*
        fn $method_name<Marker>(
            self,
            handler: impl IntoSystem<In<(Entity, $data_type)>, (), Marker> + Send + Sync + 'static,
            blocked: impl Signal<Item = bool> + Send + 'static,
        ) -> Self {
            self.with_builder(|builder| {
                builder.component_signal::<$blocked_component, _>(
                    blocked.map_true_in(|| <$blocked_component>::default()),
                )
            })
            .$blockable_method::<$blocked_component, _>(handler)
        }
    };
}

/// Macro to generate _change methods that fire only when state changes.
macro_rules! impl_change_handler {
    ($(#[$attr:meta])* $method_name:ident, $blockable_method:ident, $blocked_component:ty, $data_type:ty, $state_field:ident) => {
        $(#[$attr])*
        fn $method_name<Marker>(
            self,
            handler: impl IntoSystem<In<(Entity, $data_type)>, (), Marker> + Send + Sync + 'static,
        ) -> Self {
            let system_holder = Arc::new(OnceLock::new());
            self.with_builder(clone!((system_holder) move |builder| {
                builder
                    .on_spawn(clone!((system_holder) move |world, _| {
                        let _ = system_holder.set(register_system(world, handler));
                    }))
                    .apply(remove_system_holder_on_remove(system_holder.clone()))
            }))
            .$blockable_method::<$blocked_component, _>(
                move |In((entity, data)): In<(Entity, $data_type)>,
                      mut prev: Local<Option<bool>>,
                      mut commands: Commands| {
                    if prev.map_or(true, |prev| prev != data.$state_field) {
                        *prev = Some(data.$state_field);
                        commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data));
                    }
                },
            )
        }
    };
}

/// Macro to generate throttled methods for continuous firing with rate limiting.
/// The continuous_data_type is the type passed to handler (e.g., HoveringData),
/// while state_data_type is the type from the blockable method (e.g., HoverData).
macro_rules! impl_throttled_handler {
    (
        $(#[$attr:meta])*
        $method_name:ident,
        $blockable_method:ident,
        $blocked_component:ty,
        $timer_collection:ty,
        $state_data_type:ty,
        $continuous_data_type:ty,
        $state_field:ident,
        |$data_param:ident| $extract_continuous:expr
    ) => {
        $(#[$attr])*
        fn $method_name<Marker>(
            self,
            handler: impl IntoSystem<In<(Entity, $continuous_data_type)>, (), Marker> + Send + Sync + 'static,
            duration: Duration,
        ) -> Self {
            let system_holder = Arc::new(OnceLock::new());
            let timer_id = Arc::new(OnceLock::new());
            self.with_builder(clone!((system_holder, timer_id) move |builder| {
                builder
                    .on_spawn(
                        clone!((system_holder, timer_id) move |world, entity| {
                            let _ = system_holder.set(register_system(world, handler));

                            // Get or insert the timer collection
                            let id = if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                                if let Some(mut collection) = entity_mut.get_mut::<$timer_collection>() {
                                    collection.add_timer(duration)
                                } else {
                                    let mut collection = <$timer_collection>::default();
                                    let id = collection.add_timer(duration);
                                    entity_mut.insert(collection);
                                    id
                                }
                            } else {
                                0 // fallback, shouldn't happen
                            };
                            let _ = timer_id.set(id);
                        }),
                    )
                    .apply(remove_system_holder_on_remove(system_holder.clone()))
            }))
            .$blockable_method::<$blocked_component, _>(
                move |In((entity, data)): In<(Entity, $state_data_type)>,
                      mut commands: Commands,
                      time: Res<Time>,
                      mut collections: Query<&mut $timer_collection>| {
                    if data.$state_field {
                        if let (Ok(mut collection), Some(&id)) = (collections.get_mut(entity), timer_id.get()) {
                            if let Some(timer) = collection.get_timer_mut(id) {
                                timer.tick(time.delta());
                                if timer.is_finished() {
                                    let $data_param = &data;
                                    commands.run_system_with(
                                        system_holder.get().copied().unwrap(),
                                        (entity, $extract_continuous)
                                    );
                                    timer.reset();
                                }
                            }
                        }
                    }
                },
            )
        }
    };
}

/// Handler data for hover events, containing hover state and hit information.
#[derive(Clone)]
pub struct HoverData {
    /// Whether the element is currently hovered.
    pub hovered: bool,
    /// Hit information for the pointer intersection.
    pub hit: HitData,
    /// The pointer ID that triggered this hover event.
    pub pointer_id: PointerId,
    /// The location of the pointer during this hover event.
    pub pointer_location: Location,
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
    /// The pointer ID that triggered this press event.
    pub pointer_id: PointerId,
    /// The location of the pointer during this press event.
    pub pointer_location: Location,
}

/// Handler data for pressing events, containing button and hit information.
#[derive(Clone)]
pub struct PressingData {
    /// The button that is being pressed.
    pub button: PointerButton,
    /// Hit information for the pointer intersection.
    pub hit: HitData,
    /// The pointer ID that is being used for pressing.
    pub pointer_id: PointerId,
    /// The location of the pointer during this pressing event.
    pub pointer_location: Location,
}

/// Handler data for hovering events, containing hit information.
#[derive(Clone)]
pub struct HoveringData {
    /// Hit information for the pointer intersection.
    pub hit: HitData,
    /// The pointer ID that is hovering.
    pub pointer_id: PointerId,
    /// The location of the pointer during this hovering event.
    pub pointer_location: Location,
}

/// Handler data for drag events, containing drag state and button information.
#[derive(Clone)]
pub struct DragData {
    /// Whether the element is currently being dragged.
    pub dragged: bool,
    /// The button that was used for dragging.
    pub button: PointerButton,
    /// The pointer ID that triggered this drag event.
    pub pointer_id: PointerId,
    /// The location of the pointer during this drag event.
    pub pointer_location: Location,
    /// Hit information for the drag intersection.
    pub hit: HitData,
}

/// Handler data for dragging events, containing button information.
#[derive(Clone)]
pub struct DraggingData {
    /// The button that is being used for dragging.
    pub button: PointerButton,
    /// The pointer ID that is being used for dragging.
    pub pointer_id: PointerId,
    /// The location of the pointer during this dragging event.
    pub pointer_location: Location,
    /// The delta movement since last frame.
    pub delta: Vec2,
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
                .on_spawn(
                    clone!((hover_handler_holder, hovering_handler_holder) move |world, entity| {
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
                                        pointer_id: hover_data.pointer_id,
                                        pointer_location: hover_data.pointer_location.clone(),
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
                                let pointer_id = enter.pointer_id;
                                let pointer_location = enter.pointer_location.clone();

                                let move_observer = (!move_observers.contains(entity)).then(|| {
                                    create_move_observer!(&mut commands, entity, HoverDataInternal)
                                });

                                if let Ok(mut entity) = commands.get_entity(entity) {
                                    entity.insert(HoverDataInternal {
                                        hit: hit.clone(),
                                        pointer_id,
                                        pointer_location: pointer_location.clone(),
                                    });
                                    entity.insert(Hovered);
                                    entity.insert(HoveredSystem(hovering_handler_system));
                                    if let Some(move_observer) = move_observer {
                                        entity.insert(HoverMoveObserver(move_observer));
                                    }
                                }

                                commands.run_system_with(hover_handler_system, (entity, HoverData {
                                    hovered: true,
                                    hit,
                                    pointer_id,
                                    pointer_location,
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
                                let pointer_id = leave.pointer_id;
                                let pointer_location = leave.pointer_location.clone();

                                if !blocked.contains(entity) {
                                    commands.run_system_with(hover_handler_system, (entity, HoverData {
                                        hovered: false,
                                        hit,
                                        pointer_id,
                                        pointer_location,
                                    }));
                                }

                                let move_observer = move_observers.get(entity).ok().map(|o| o.0);

                                if let Ok(mut entity) = commands.get_entity(entity) {
                                    entity.remove::<HoverDataInternal>();
                                    entity.remove::<HoveredSystem>();
                                    if move_observer.is_some() {
                                        entity.remove::<HoverMoveObserver>();
                                    }
                                }

                                if let Some(observer) = move_observer {
                                    commands.entity(observer).despawn();
                                }
                            },
                        );
                    }),
                )
                .apply(remove_system_holder_on_remove(hover_handler_holder))
                .apply(remove_system_holder_on_remove(hovering_handler_holder))
        })
    }

    impl_blockable_signal!(
        #[doc = "Like [`PointerEventAware::on_hovered_blockable`], but reactively controls whether hover handling is blocked with a [`Signal`]."]
        on_hovered_blockable_signal,
        on_hovered_blockable,
        HoverHandlingBlocked,
        HoverData
    );

    impl_change_handler!(
        #[doc = "When this element's hovered state changes, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`HoverHandlerData`]. This method can be called repeatedly to register many such handlers."]
        on_hovered_change,
        on_hovered_blockable,
        HoverHandlingBlocked,
        HoverData,
        hovered
    );

    /// On frames where this element is hovered and does not have a `Blocked` [`Component`], run a
    /// [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`HoveringData`].
    /// This method can be called repeatedly to register many such handlers.
    fn on_hovering_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoveringData)>, (), Marker> + Send + Sync + 'static,
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
                    commands.run_system_with(
                        system_holder.get().copied().unwrap(),
                        (
                            entity,
                            HoveringData {
                                hit: data.hit,
                                pointer_id: data.pointer_id,
                                pointer_location: data.pointer_location,
                            },
                        ),
                    );
                }
            },
        )
    }

    impl_blockable_signal!(
        #[doc = "Like [`PointerEventAware::on_hovering_blockable`], but reactively controls whether hover handling is blocked with a [`Signal`]."]
        on_hovering_blockable_signal,
        on_hovering_blockable,
        HoverHandlingBlocked,
        HoveringData
    );

    /// On frames where this element is hovered, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HoveringData`].
    fn on_hovering<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoveringData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_hovering_blockable::<HoverHandlingBlocked, _>(handler)
    }

    impl_throttled_handler!(
        #[doc = "On frames where this element is hovered, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`HoveringData`], throttled by `duration` before the `handler` can run again."]
        on_hovering_throttled,
        on_hovered_blockable,
        HoverHandlingBlocked,
        HoverThrottleTimers,
        HoverData,
        HoveringData,
        hovered,
        |data| HoveringData {
            hit: data.hit.clone(),
            pointer_id: data.pointer_id,
            pointer_location: data.pointer_location.clone(),
        }
    );

    /// On frames where this element is clicked and does not have a `Blocked` [`Component`],
    /// run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and
    /// [`Pointer<Click>`]. This method can be called repeatedly to register many such handlers.
    fn on_click_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |click: On<Pointer<Click>>, blocked: Query<&Blocked>, mut commands: Commands| {
                        if blocked.contains(click.entity) {
                            return;
                        }
                        commands.run_system_with(system, (click.entity, (*click).clone()));
                    });
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    /// Run a [`System`] when this element is clicked.
    fn on_click<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_click_blockable::<ClickHandlingBlocked, _>(handler)
    }

    impl_blockable_signal!(
        #[doc = "Like [`PointerEventAware::on_click_blockable`], but reactively controls whether click handling is blocked with a [`Signal`]."]
        on_click_blockable_signal,
        on_click_blockable,
        ClickHandlingBlocked,
        Pointer<Click>
    );

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents and the element does not have a `Blocked` [`Component`], run a
    /// [`System`] that takes [`In`](`System::In`) this element's [`Entity`] and the
    /// [`Pointer<Click>`]. Will not function unless this element is a descendant of a [`UiRoot`].
    /// This method can be called repeatedly to register many such handlers.
    fn on_click_outside_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, GlobalEventData<Pointer<Click>>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_remove(system_holder.clone()))
        })
        .on_global_event::<Pointer<Click>, _, _, _>(
            move |In((entity, click)): In<(Entity, GlobalEventData<Pointer<Click>>)>,
                  childrens: Query<&Children>,
                  child_ofs: Query<&ChildOf>,
                  ui_roots: Query<&UiRoot>,
                  blocked: Query<&Blocked>,
                  mut commands: Commands| {
                if blocked.contains(entity) {
                    return;
                }
                for ancestor in child_ofs.iter_ancestors(entity) {
                    if ui_roots.contains(ancestor) {
                        if !is_inside_or_removed_from_dom(entity, &click, ancestor, &childrens) {
                            commands.run_system_with(system_holder.get().copied().unwrap(), (entity, click));
                        }
                        break;
                    }
                }
            },
        )
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a [`System`] that takes [`In`](`System::In`) this element's
    /// [`Entity`] and the [`Pointer<Click>`]. Will not function unless this element is a descendant
    /// of a [`UiRoot`]. This method can be called repeatedly to register many such handlers.
    fn on_click_outside<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, GlobalEventData<Pointer<Click>>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_click_outside_blockable::<ClickOutsideHandlingBlocked, _>(handler)
    }

    impl_blockable_signal!(
        #[doc = "Like [`PointerEventAware::on_click_outside_blockable`], but reactively controls whether click outside handling is blocked with a [`Signal`]."]
        on_click_outside_blockable_signal,
        on_click_outside_blockable,
        ClickOutsideHandlingBlocked,
        GlobalEventData<Pointer<Click>>
    );

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
                .on_spawn(
                    clone!((press_handler_holder, pressing_handler_holder) move |world, entity| {
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
                                            pointer_id: press_data.pointer_id,
                                            pointer_location: press_data.pointer_location.clone(),
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
                                let pointer_id = press.pointer_id;
                                let pointer_location = press.pointer_location.clone();

                                let move_observer = (!move_observers.contains(entity)).then(|| {
                                    create_move_observer!(&mut commands, entity, PressDataInternal)
                                });

                                if let Ok(mut entity) = commands.get_entity(entity) {
                                    entity.insert(PressDataInternal {
                                        button,
                                        hit: hit.clone(),
                                        pointer_id,
                                        pointer_location: pointer_location.clone(),
                                    });
                                    entity.insert(PressedSystem(pressing_handler_system));
                                    if let Some(move_observer) = move_observer {
                                        entity.insert(PressMoveObserver(move_observer));
                                    }
                                }

                                commands.run_system_with(press_handler_system, (entity, PressData {
                                    pressed: true,
                                    button,
                                    hit,
                                    pointer_id,
                                    pointer_location,
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
                                let pointer_id = release.pointer_id;
                                let pointer_location = release.pointer_location.clone();

                                let move_observer = move_observers.get(entity).ok().map(|o| o.0);

                                if !blocked.contains(entity) {
                                    commands.run_system_with(press_handler_system, (entity, PressData {
                                        pressed: false,
                                        button,
                                        hit,
                                        pointer_id,
                                        pointer_location,
                                    }));
                                }

                                if let Ok(mut entity) = commands.get_entity(entity) {
                                    entity.remove::<PressDataInternal>();
                                    entity.remove::<PressedSystem>();
                                    if move_observer.is_some() {
                                        entity.remove::<PressMoveObserver>();
                                    }
                                }

                                if let Some(observer) = move_observer {
                                    commands.entity(observer).despawn();
                                }
                            },
                        );
                    }),
                )
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
                    if let Ok(mut entity) = commands.get_entity(entity) {
                        entity.remove::<PressMoveObserver>();
                        entity.remove::<PressDataInternal>();
                        entity.remove::<PressedSystem>();
                    }
                }
            },
        )
    }

    impl_blockable_signal!(
        #[doc = "Like [`PointerEventAware::on_pressed_blockable`], but reactively controls whether press handling is blocked with a [`Signal`]."]
        on_pressed_blockable_signal,
        on_pressed_blockable,
        PressHandlingBlocked,
        PressData
    );

    impl_change_handler!(
        #[doc = "When this element's pressed state changes, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`PressHandlerData`]. This method can be called repeatedly to register many such handlers."]
        on_pressed_change,
        on_pressed_blockable,
        PressHandlingBlocked,
        PressData,
        pressed
    );

    /// On frames where this element is being pressed and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`] and [`PressingData`]. This method can be called repeatedly
    /// to register many such handlers.
    fn on_pressing_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressingData)>, (), Marker> + Send + Sync + 'static,
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
                    commands.run_system_with(
                        system_holder.get().copied().unwrap(),
                        (
                            entity,
                            PressingData {
                                button: data.button,
                                hit: data.hit,
                                pointer_id: data.pointer_id,
                                pointer_location: data.pointer_location,
                            },
                        ),
                    );
                }
            },
        )
    }

    impl_blockable_signal!(
        #[doc = "On frames where this element is being pressed, run a [`System`], reactively controlling whether the press is blocked with a [`Signal`]."]
        on_pressing_blockable_signal,
        on_pressing_blockable,
        PressHandlingBlocked,
        PressingData
    );

    /// When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`] and [`PressingData`].
    fn on_pressing<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressingData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_pressing_blockable::<PressHandlingBlocked, _>(handler)
    }

    impl_throttled_handler!(
        #[doc = "When this element is being pressed, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`PressingData`], throttled by `duration` before the `handler` can run again."]
        on_pressing_throttled,
        on_pressed_blockable,
        PressHandlingBlocked,
        PressThrottleTimers,
        PressData,
        PressingData,
        pressed,
        |data| PressingData {
            button: data.button,
            hit: data.hit.clone(),
            pointer_id: data.pointer_id,
            pointer_location: data.pointer_location.clone(),
        }
    );

    /// On frames where this element is dragged or gets undragged and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and [`DragData`]. This method can be called repeatedly to register many such handlers.
    fn on_dragged_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let drag_handler_holder = Arc::new(OnceLock::new());
            builder
                .on_spawn(clone!((drag_handler_holder) move |world, entity| {
                    let drag_handler_system = register_system(world, handler);
                    let _ = drag_handler_holder.set(drag_handler_system);

                    observe(
                        world,
                        entity,
                        move |drag_start: On<Pointer<DragStart>>,
                              blocked: Query<&Blocked>,
                              drag_observers: Query<&DragMoveObserver>,
                              mut commands: Commands| {
                            let entity = drag_start.entity;
                            if blocked.contains(entity) {
                                return;
                            }

                            let button = drag_start.button;
                            let hit = drag_start.hit.clone();
                            let pointer_id = drag_start.pointer_id;
                            let pointer_location = drag_start.pointer_location.clone();

                            let drag_observer = (!drag_observers.contains(entity)).then(|| {
                                create_move_observer!(&mut commands, entity, DragDataInternal)
                            });

                            if let Ok(mut entity) = commands.get_entity(entity) {
                                entity.insert(DragDataInternal {
                                    button,
                                    hit: hit.clone(),
                                    pointer_id,
                                    pointer_location: pointer_location.clone(),
                                    delta: Vec2::ZERO,
                                });
                                entity.insert(Dragged);
                                if let Some(drag_observer) = drag_observer {
                                    entity.insert(DragMoveObserver(drag_observer));
                                }
                            }

                            commands.run_system_with(drag_handler_system, (entity, DragData {
                                dragged: true,
                                button,
                                hit,
                                pointer_id,
                                pointer_location,
                            }));
                        },
                    );

                    observe(
                        world,
                        entity,
                        move |drag_end: On<Pointer<DragEnd>>,
                              blocked: Query<&Blocked>,
                              drag_observers: Query<&DragMoveObserver>,
                              drag_datas: Query<&DragDataInternal>,
                              mut commands: Commands| {
                            let entity = drag_end.entity;

                            let stored_hit = if let Ok(drag_data) = drag_datas.get(entity) {
                                if drag_data.button != drag_end.button {
                                    return;
                                }
                                drag_data.hit.clone()
                            } else {
                                return;
                            };

                            let button = drag_end.button;
                            let pointer_id = drag_end.pointer_id;
                            let pointer_location = drag_end.pointer_location.clone();

                            let drag_observer = drag_observers.get(entity).ok().map(|o| o.0);

                            if !blocked.contains(entity) {
                                commands.run_system_with(drag_handler_system, (entity, DragData {
                                    dragged: false,
                                    button,
                                    hit: stored_hit,
                                    pointer_id,
                                    pointer_location,
                                }));
                            }

                            if let Ok(mut entity) = commands.get_entity(entity) {
                                entity.remove::<DragDataInternal>();
                                entity.remove::<Dragged>();
                                if drag_observer.is_some() {
                                    entity.remove::<DragMoveObserver>();
                                }
                            }

                            if let Some(observer) = drag_observer {
                                commands.entity(observer).despawn();
                            }
                        },
                    );
                }))
                .apply(remove_system_holder_on_remove(drag_handler_holder))
        })
    }

    impl_blockable_signal!(
        #[doc = "Like [`PointerEventAware::on_dragged_blockable`], but reactively controls whether drag handling is blocked with a [`Signal`]."]
        on_dragged_blockable_signal,
        on_dragged_blockable,
        DragHandlingBlocked,
        DragData
    );

    impl_change_handler!(
        #[doc = "When this element's dragged state changes, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`DragData`]. This method can be called repeatedly to register many such handlers."]
        on_dragged_change,
        on_dragged_blockable,
        DragHandlingBlocked,
        DragData,
        dragged
    );

    /// On frames where this element is being dragged and does not have a `Blocked`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's
    /// [`Entity`] and [`DraggingData`]. This method can be called repeatedly
    /// to register many such handlers.
    fn on_dragging_blockable<Blocked: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DraggingData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let dragging_handler_holder = Arc::new(OnceLock::new());
            builder
                .on_spawn(clone!((dragging_handler_holder) move |world, entity| {
                    let dragging_handler = register_system(world, handler);
                    let _ = dragging_handler_holder.set(dragging_handler);

                    let per_entity_dragging_system = world.register_system(
                        move |In(entity): In<Entity>,
                              mut commands: Commands,
                              blocked_query: Query<(), With<Blocked>>,
                              drag_state_query: Query<&DragDataInternal>| {
                            if blocked_query.contains(entity) {
                                return;
                            }
                            if let Ok(drag_data) = drag_state_query.get(entity) {
                                let dragging_data = DraggingData {
                                    button: drag_data.button,
                                    pointer_id: drag_data.pointer_id,
                                    pointer_location: drag_data.pointer_location.clone(),
                                    delta: drag_data.delta,
                                };
                                commands.run_system_with(dragging_handler, (entity, dragging_data));
                            }
                        },
                    );

                    observe(
                        world,
                        entity,
                        move |drag_start: On<Pointer<DragStart>>,
                              blocked: Query<&Blocked>,
                              drag_observers: Query<&DragMoveObserver>,
                              dragged_systems: Query<&DraggedSystem>,
                              mut commands: Commands| {
                            let entity = drag_start.entity;
                            if blocked.contains(entity) {
                                return;
                            }

                            let button = drag_start.button;
                            let hit = drag_start.hit.clone();
                            let pointer_id = drag_start.pointer_id;
                            let pointer_location = drag_start.pointer_location.clone();

                            let drag_observer = (!drag_observers.contains(entity)).then(|| {
                                commands
                                    .spawn((
                                        Observer::new(
                                            move |drag_event: On<Pointer<Drag>>,
                                                  mut drag_datas: Query<&mut DragDataInternal>| {
                                                let entity = drag_event.entity;
                                                if let Ok(mut drag_data) = drag_datas.get_mut(entity)
                                                    && drag_data.button == drag_event.button {
                                                        drag_data.pointer_location = drag_event.pointer_location.clone();
                                                        drag_data.delta = drag_event.delta;
                                                    }
                                            },
                                        )
                                        .with_entity(entity),
                                        HaalkaObserver,
                                    ))
                                    .id()
                            });

                            if let Ok(mut entity) = commands.get_entity(entity) {
                                entity.insert(DragDataInternal {
                                    button,
                                    hit: hit.clone(),
                                    pointer_id,
                                    pointer_location: pointer_location.clone(),
                                    delta: Vec2::ZERO,
                                });
                                entity.insert(Dragged);
                                if let Some(drag_observer) = drag_observer {
                                    entity.insert(DragMoveObserver(drag_observer));
                                }
                                if !dragged_systems.contains(entity.id()) {
                                    entity.insert(DraggedSystem(per_entity_dragging_system));
                                }
                            }
                        },
                    );

                    observe(
                        world,
                        entity,
                        move |drag_end: On<Pointer<DragEnd>>,
                              drag_observers: Query<&DragMoveObserver>,
                              drag_datas: Query<&DragDataInternal>,
                              mut commands: Commands| {
                            let entity = drag_end.entity;

                            if let Ok(drag_data) = drag_datas.get(entity) {
                                if drag_data.button != drag_end.button {
                                    return;
                                }
                            } else {
                                return;
                            }

                            let drag_observer = drag_observers.get(entity).ok().map(|o| o.0);

                            if let Ok(mut entity) = commands.get_entity(entity) {
                                entity.remove::<DragDataInternal>();
                                entity.remove::<Dragged>();
                                entity.remove::<DraggedSystem>();
                                if drag_observer.is_some() {
                                    entity.remove::<DragMoveObserver>();
                                }
                            }

                            if let Some(observer) = drag_observer {
                                commands.entity(observer).despawn();
                            }
                        },
                    );
                }))
                .apply(remove_system_holder_on_remove(dragging_handler_holder))
        })
    }

    impl_blockable_signal!(
        #[doc = "On frames where this element is being dragged, run a [`System`], reactively controlling whether the drag is blocked with a [`Signal`]."]
        on_dragging_blockable_signal,
        on_dragging_blockable,
        DragHandlingBlocked,
        DraggingData
    );

    /// When this element is being dragged, run a [`System`] which takes [`In`](`System::In`) this
    /// element's [`Entity`] and [`DraggingData`].
    fn on_dragging<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DraggingData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_dragging_blockable::<DragHandlingBlocked, _>(handler)
    }

    impl_throttled_handler!(
        #[doc = "When this element is being dragged, run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`DraggingData`], throttled by `duration` before the `handler` can run again."]
        on_dragging_throttled,
        on_dragged_blockable,
        DragHandlingBlocked,
        DragThrottleTimers,
        DragData,
        DraggingData,
        dragged,
        |data| DraggingData {
            button: data.button,
            pointer_id: data.pointer_id,
            pointer_location: data.pointer_location.clone(),
            delta: Vec2::ZERO,
        }
    );
}

/// Timer collection component for throttling press events.
#[derive(Component, Default)]
struct PressThrottleTimers {
    timers: std::collections::HashMap<usize, Timer>,
    next_id: usize,
}

impl PressThrottleTimers {
    fn add_timer(&mut self, duration: Duration) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.timers.insert(id, Timer::new(duration, TimerMode::Once));
        id
    }

    fn get_timer_mut(&mut self, id: usize) -> Option<&mut Timer> {
        self.timers.get_mut(&id)
    }
}

/// Timer collection component for throttling hover events.
#[derive(Component, Default)]
struct HoverThrottleTimers {
    timers: std::collections::HashMap<usize, Timer>,
    next_id: usize,
}

impl HoverThrottleTimers {
    fn add_timer(&mut self, duration: Duration) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.timers.insert(id, Timer::new(duration, TimerMode::Once));
        id
    }

    fn get_timer_mut(&mut self, id: usize) -> Option<&mut Timer> {
        self.timers.get_mut(&id)
    }
}

#[derive(Component, Clone)]
pub struct Hovered;

#[derive(Component, Clone)]
pub struct Dragged;

#[derive(Component, Default, Clone)]
struct PressHandlingBlocked;

#[derive(Component, Default, Clone)]
struct HoverHandlingBlocked;

#[derive(Component, Default, Clone)]
struct DragHandlingBlocked;

#[derive(Component, Default, Clone)]
struct ClickHandlingBlocked;

#[derive(Component, Default, Clone)]
struct ClickOutsideHandlingBlocked;

/// Timer collection component for throttling drag events.
#[derive(Component, Default)]
struct DragThrottleTimers {
    timers: std::collections::HashMap<usize, Timer>,
    next_id: usize,
}

impl DragThrottleTimers {
    fn add_timer(&mut self, duration: Duration) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.timers.insert(id, Timer::new(duration, TimerMode::Once));
        id
    }

    fn get_timer_mut(&mut self, id: usize) -> Option<&mut Timer> {
        self.timers.get_mut(&id)
    }
}

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

#[derive(Component)]
struct OutPropagationStopped;

#[derive(Component, Clone)]
struct PressDataInternal {
    button: PointerButton,
    hit: HitData,
    pointer_id: PointerId,
    pointer_location: Location,
}

impl PointerDataInternal for PressDataInternal {
    fn update_from_move(&mut self, hit: HitData, pointer_location: Location) {
        self.hit = hit;
        self.pointer_location = pointer_location;
    }
}

#[derive(Component, Clone, Copy)]
struct PressMoveObserver(Entity);

#[derive(Component, Clone)]
struct HoverDataInternal {
    hit: HitData,
    pointer_id: PointerId,
    pointer_location: Location,
}

impl PointerDataInternal for HoverDataInternal {
    fn update_from_move(&mut self, hit: HitData, pointer_location: Location) {
        self.hit = hit;
        self.pointer_location = pointer_location;
    }
}

#[derive(Component, Clone, Copy)]
struct HoverMoveObserver(Entity);

#[derive(Component, Clone, Copy)]
struct HoveredSystem(SystemId<In<Entity>, ()>);

#[derive(Component)]
pub(crate) struct Hoverable;

#[derive(Component)]
pub(crate) struct Pressable;

#[derive(Component)]
struct PressedSystem(SystemId<In<Entity>, ()>);

#[derive(Component, Clone)]
struct DragDataInternal {
    button: PointerButton,
    hit: HitData,
    pointer_id: PointerId,
    pointer_location: Location,
    delta: Vec2,
}

impl PointerDataInternal for DragDataInternal {
    fn update_from_move(&mut self, hit: HitData, pointer_location: Location) {
        self.hit = hit;
        self.pointer_location = pointer_location;
    }
}

#[derive(Component, Clone, Copy)]
struct DragMoveObserver(Entity);

#[derive(Component, Clone, Copy)]
struct DraggedSystem(SystemId<In<Entity>, ()>);

fn pressed_system(mut interaction_query: Query<(Entity, &PressedSystem), With<Pressed>>, mut commands: Commands) {
    for (entity, &PressedSystem(system)) in &mut interaction_query {
        commands.run_system_with(system, entity);
    }
}

fn dragged_system(mut interaction_query: Query<(Entity, &DraggedSystem), With<Dragged>>, mut commands: Commands) {
    for (entity, &DraggedSystem(system)) in &mut interaction_query {
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

fn hovered_system(mut hovering_query: Query<(Entity, &HoveredSystem), With<Hovered>>, mut commands: Commands) {
    for (entity, &HoveredSystem(system)) in &mut hovering_query {
        commands.run_system_with(system, entity);
    }
}

fn contains(left: Entity, right: Entity, childrens: &Query<&Children>) -> bool {
    left == right || childrens.iter_descendants(left).any(|e| e == right)
}

// TODO: add support for some sort of exclusion
// ported from moonzoon https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/ability/mouse_event_aware.rs#L158
fn is_inside_or_removed_from_dom(
    element: Entity,
    event: &GlobalEventData<Pointer<Click>>,
    ui_root: Entity,
    childrens: &Query<&Children>,
) -> bool {
    if contains(element, event.original_event_target, childrens) {
        return true;
    }
    if !contains(ui_root, event.original_event_target, childrens) {
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
                .insert(CursorOverPropagationStopped)
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
                        if let Ok(mut entity) = commands.get_entity(entity) {
                            entity.try_insert(CursorOverPropagationStopped);
                            if cursor_over.get(entity.id()).is_ok() {
                                entity.try_insert(CursorOver);
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
            builder.component_signal::<CursorOnHover, _>(
                cursor_option_signal.map_in(|into_option_cursor| Some(CursorOnHover(into_option_cursor.into()))),
            )
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
            builder.component_signal::<CursorDisabled, _>(disabled.map_true_in(|| CursorDisabled))
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
        self.cursor_signal_disableable_signal(SignalBuilder::always(cursor_option.into()).first(), disabled)
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
            dragged_system.run_if(any_with_component::<DraggedSystem>),
            (
                update_hover_states.run_if(
                    any_with_component::<Hoverable>
                        // TODO: apparently this updates every frame no matter what, if so, remove this condition
                        // TODO: remove when native `Enter` and `Leave` available
                        .and(resource_exists_and_changed::<HoverMap>)
                        .and(not(resource_exists::<UpdateHoverStatesDisabled>)),
                ),
                hovered_system.run_if(any_with_component::<HoveredSystem>),
            )
                .chain(),
            consume_queued_cursor.run_if(resource_removed::<CursorOnHoverDisabled>),
        ),
    );
}
