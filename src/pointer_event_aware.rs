//! Semantics for managing how an [`Element`](super::element::Element) reacts to pointer events like
//! hover, click, and press, see [`PointerEventAware`].

use std::{
    ops::Not,
    sync::{Arc, OnceLock},
    time::Duration,
};

use apply::Apply;
use bevy_app::prelude::*;
use bevy_ecs::{component::Mutable, lifecycle::HookContext, prelude::*, system::SystemId, world::DeferredWorld};
use bevy_log::prelude::*;
use bevy_math::Vec2;
use bevy_picking::{
    backend::prelude::*,
    hover::{HoverMap, PickingInteraction},
    pointer::{Location, PointerMap},
    prelude::*,
};
use bevy_platform::collections::HashMap;
use bevy_reflect::prelude::*;
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::Pressed;
use bevy_window::*;
use jonmo::prelude::*;

use super::{
    element::UiRoot,
    global_event_aware::{GlobalEventAware, GlobalEventData},
    utils::{HaalkaObserver, clone, observe, register_system, remove_system_holder_on_despawn},
};

/// Helper trait for internal data components that track pointer state.
trait PointerDataInternal {
    fn update_from_move(&mut self, hit: HitData, pointer_location: Location);
}

/// Create a move observer for a specific data type that implements [`PointerDataInternal`].
fn create_move_observer<D: PointerDataInternal + Component<Mutability = Mutable>>(
    commands: &mut Commands,
    entity: Entity,
) -> Entity {
    commands
        .spawn((
            Observer::new(|move_event: On<Pointer<Move>>, mut datas: Query<&mut D>| {
                let entity = move_event.entity;
                if let Ok(mut data) = datas.get_mut(entity) {
                    data.update_from_move(move_event.hit.clone(), move_event.pointer_location.clone());
                }
            })
            .with_entity(entity),
            HaalkaObserver,
        ))
        .id()
}

/// Component storing signal-based disabling state flags.
#[derive(Component, Default)]
pub(crate) struct DisableableSignalState {
    pub(crate) flags: Vec<bool>,
}

/// Timer collection component for throttling events.
#[derive(Component, Default)]
struct ThrottleTimers {
    timers: HashMap<usize, Timer>,
    was_active: HashMap<usize, bool>,
    next_id: usize,
}

impl ThrottleTimers {
    fn add_timer(&mut self, duration: Duration) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.timers.insert(id, Timer::new(duration, TimerMode::Once));
        self.was_active.insert(id, false);
        id
    }

    /// Add a timer that starts finished, so the first event fires immediately.
    fn add_timer_ready(&mut self, duration: Duration) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let mut timer = Timer::new(duration, TimerMode::Once);
        timer.tick(duration); // Start finished
        self.timers.insert(id, timer);
        id
    }

    /// Returns true if the handler should fire.
    ///
    /// Fires immediately on activation (false → true transition), then throttles while active.
    /// Resets the timer when deactivated so the next activation fires immediately.
    fn tick_and_check(&mut self, id: usize, delta: std::time::Duration, is_active: bool) -> bool {
        let was_active = self.was_active.get(&id).copied().unwrap_or(false);
        self.was_active.insert(id, is_active);

        if !is_active {
            // Reset timer on deactivation so next activation fires immediately
            if let Some(timer) = self.timers.get_mut(&id) {
                timer.reset();
                // Finish the timer so the next check passes immediately
                timer.tick(timer.duration());
            }
            return false;
        }

        // New activation, fire immediately
        if !was_active {
            if let Some(timer) = self.timers.get_mut(&id) {
                timer.reset();
            }
            return true;
        }

        // Continuing to be active, throttle
        if let Some(timer) = self.timers.get_mut(&id)
            && timer.tick(delta).is_finished() {
                timer.reset();
                return true;
            }
        false
    }

    /// For discrete events (like clicks): returns true if enough time has passed since last fire.
    /// Ticks the timer and checks if it's finished. If so, resets and returns true.
    fn check_discrete(&mut self, id: usize, delta: std::time::Duration) -> bool {
        if let Some(timer) = self.timers.get_mut(&id) {
            timer.tick(delta);
            if timer.is_finished() {
                timer.reset();
                return true;
            }
        }
        false
    }
}

/// Helper to register a throttle timer on an entity during spawn.
fn add_throttle_timer(world: &mut World, entity: Entity, duration: Duration) -> usize {
    let mut entity = world.entity_mut(entity);
    if let Some(mut collection) = entity.get_mut::<ThrottleTimers>() {
        collection.add_timer(duration)
    } else {
        let mut collection = ThrottleTimers::default();
        let id = collection.add_timer(duration);
        entity.insert(collection);
        id
    }
}

/// Helper to register a throttle timer that starts ready (first event fires immediately).
fn add_throttle_timer_ready(world: &mut World, entity: Entity, duration: Duration) -> usize {
    let mut entity = world.entity_mut(entity);
    if let Some(mut collection) = entity.get_mut::<ThrottleTimers>() {
        collection.add_timer_ready(duration)
    } else {
        let mut collection = ThrottleTimers::default();
        let id = collection.add_timer_ready(duration);
        entity.insert(collection);
        id
    }
}

/// Returns a builder function that registers a handler system on spawn for change detection.
#[allow(clippy::type_complexity)]
fn change_setup<D: 'static, Marker>(
    handler: impl IntoSystem<In<(Entity, D)>, (), Marker> + Send + Sync + 'static,
    system_holder: Arc<OnceLock<SystemId<In<(Entity, D)>, ()>>>,
) -> impl FnOnce(jonmo::Builder) -> jonmo::Builder {
    move |builder| {
        builder
            .on_spawn(clone!((system_holder) move |world, _| {
                let _ = system_holder.set(register_system(world, handler));
            }))
            .apply(remove_system_holder_on_despawn(system_holder))
    }
}

/// Creates a change detection system that fires when the boolean field changes.
#[allow(clippy::type_complexity)]
fn change_system<D: Clone + Send + Sync + 'static>(
    system_holder: Arc<OnceLock<SystemId<In<(Entity, D)>, ()>>>,
    get_field: fn(&D) -> bool,
) -> impl FnMut(In<(Entity, D)>, Local<Option<bool>>, Commands) + Send + Sync + 'static {
    move |In((entity, data)): In<(Entity, D)>, mut prev: Local<Option<bool>>, mut commands: Commands| {
        let field = get_field(&data);
        if prev.is_none_or(|prev| prev != field) {
            *prev = Some(field);
            commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data));
        }
    }
}

/// Returns a builder function that registers a handler system and throttle timer on spawn.
#[allow(clippy::type_complexity)]
fn throttle_setup<D: 'static, Marker>(
    handler: impl IntoSystem<In<(Entity, D)>, (), Marker> + Send + Sync + 'static,
    duration: Duration,
    system_holder: Arc<OnceLock<SystemId<In<(Entity, D)>, ()>>>,
    timer_id: Arc<OnceLock<usize>>,
) -> impl FnOnce(jonmo::Builder) -> jonmo::Builder {
    move |builder| {
        builder
            .on_spawn(clone!((system_holder, timer_id) move |world, entity| {
                let _ = system_holder.set(register_system(world, handler));
                let _ = timer_id.set(add_throttle_timer(world, entity, duration));
            }))
            .apply(remove_system_holder_on_despawn(system_holder))
    }
}

/// Creates a throttled system that fires immediately on activation, then throttles while active.
#[allow(clippy::type_complexity)]
fn throttle_system<D: Clone + Send + Sync + 'static>(
    system_holder: Arc<OnceLock<SystemId<In<(Entity, D)>, ()>>>,
    timer_id: Arc<OnceLock<usize>>,
    get_field: fn(&D) -> bool,
) -> impl FnMut(In<(Entity, D)>, Commands, Res<Time>, Query<&mut ThrottleTimers>) + Send + Sync + 'static {
    move |In((entity, data)): In<(Entity, D)>,
          mut commands: Commands,
          time: Res<Time>,
          mut collections: Query<&mut ThrottleTimers>| {
        let is_active = get_field(&data);
        if let (Ok(mut collection), Some(&id)) = (collections.get_mut(entity), timer_id.get())
            && collection.tick_and_check(id, time.delta(), is_active) {
                commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data));
            }
    }
}

/// Returns a builder function that sets up signal-based disabling for a handler.
#[allow(clippy::type_complexity)]
pub(crate) fn disableable_signal_setup<D: 'static, Marker>(
    handler: impl IntoSystem<In<(Entity, D)>, (), Marker> + Send + Sync + 'static,
    disabled: impl Signal<Item = bool> + 'static,
    system_holder: Arc<OnceLock<SystemId<In<(Entity, D)>, ()>>>,
    state_index: Arc<OnceLock<usize>>,
) -> impl FnOnce(jonmo::Builder) -> jonmo::Builder {
    move |builder| {
        builder
            .on_spawn(clone!((system_holder, state_index) move |world, entity| {
                let _ = system_holder.set(register_system(world, handler));

                let mut entity = world.entity_mut(entity);
                let index = if let Some(mut state) = entity.get_mut::<DisableableSignalState>() {
                    let index = state.flags.len();
                    state.flags.push(false);
                    index
                } else {
                    entity.insert(DisableableSignalState { flags: vec![false] });
                    0
                };
                let _ = state_index.set(index);
            }))
            .on_signal_with_entity(
                disabled,
                clone!((state_index) move |mut entity, disabled| {
                    let Some(index) = state_index.get().copied() else {
                        return;
                    };
                    if let Some(mut state) = entity.get_mut::<DisableableSignalState>()
                        && let Some(flag) = state.flags.get_mut(index) {
                            *flag = disabled;
                        }
                }),
            )
            .apply(remove_system_holder_on_despawn(system_holder))
    }
}

/// Creates a system that checks signal-based disabling before running the handler.
#[allow(clippy::type_complexity)]
pub(crate) fn disableable_signal_system<D: Send + Sync + 'static>(
    system_holder: Arc<OnceLock<SystemId<In<(Entity, D)>, ()>>>,
    state_index: Arc<OnceLock<usize>>,
) -> impl FnMut(In<(Entity, D)>, Query<&DisableableSignalState>, Commands) + Send + Sync + 'static {
    move |In((entity, data)): In<(Entity, D)>, states: Query<&DisableableSignalState>, mut commands: Commands| {
        if let Some(index) = state_index.get().copied()
            && let Ok(state) = states.get(entity)
                && *state.flags.get(index).unwrap_or(&false) {
                    return;
                }
        commands.run_system_with(system_holder.get().copied().unwrap(), (entity, data));
    }
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
    /// The delta movement since last frame.
    pub delta: Vec2,
}

/// Enables [`Pickable`] elements to react to pointer events like hover, click, and press.
///
/// **Note:** These methods will only function for [`Pickable`] entities (e.g., those with
/// [`Pickable::default()`]).
///
/// Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`PointerEventAware`](https://github.com/MoonZoon/MoonZoon/blob/19c6cf6b4d07cd27bee7758977ef1ea4d5b9933d/crates/zoon/src/element/ability/pointer_event_aware.rs).
pub trait PointerEventAware: GlobalEventAware {
    /// On frames where this element is hovered or gets unhovered and does not have a `Disabled`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and [`HoverData`] containing its current hovered state and the latest [`HitData`].
    /// While hovered, the handler will be executed every frame. This method can be called
    /// repeatedly to register many such handlers.
    fn on_hovered_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let hover_handler_holder = Arc::new(OnceLock::new());
            let hovering_handler_holder = Arc::new(OnceLock::new());
            builder
                .insert(Hoverable)
                .on_spawn(
                    clone!((hover_handler_holder, hovering_handler_holder) move |world, entity| {
                        let hover_handler_system = register_system(world, handler);
                        let _ = hover_handler_holder.set(hover_handler_system);

                        let hovering_handler_system = register_system(
                            world,
                            move |In(entity): In<Entity>,
                                  hover_datas: Query<&HoverDataInternal>,
                                  disabled: Query<&Disabled>,
                                  mut commands: Commands| {
                                if disabled.contains(entity) {
                                    return;
                                }
                                if let Ok(hover_data) = hover_datas.get(entity) {
                                    commands.run_system_with(
                                        hover_handler_system,
                                        (entity, HoverData {
                                            hovered: true,
                                            hit: hover_data.hit.clone(),
                                            pointer_id: hover_data.pointer_id,
                                            pointer_location: hover_data.pointer_location.clone(),
                                        }),
                                    );
                                }
                            },
                        );
                        let _ = hovering_handler_holder.set(hovering_handler_system);

                        observe(
                            world,
                            entity,
                            move |mut enter: On<Pointer<Enter>>,
                                  disabled: Query<&Disabled>,
                                  move_observers: Query<&HoverMoveObserver>,
                                  mut commands: Commands| {
                                enter.propagate(false);

                                let entity = enter.entity;
                                if disabled.contains(entity) {
                                    return;
                                }

                                let hit = enter.hit.clone();
                                let pointer_id = enter.pointer_id;
                                let pointer_location = enter.pointer_location.clone();

                                let move_observer = (!move_observers.contains(entity)).then(|| {
                                    create_move_observer::<HoverDataInternal>(&mut commands, entity)
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

                                commands.run_system_with(
                                    hover_handler_system,
                                    (entity, HoverData {
                                        hovered: true,
                                        hit,
                                        pointer_id,
                                        pointer_location,
                                    }),
                                );
                            },
                        );

                        observe(
                            world,
                            entity,
                                move |mut leave: On<Pointer<Leave>>,
                                    disabled: Query<&Disabled>,
                                    move_observers: Query<&HoverMoveObserver>,
                                    mut commands: Commands| {
                                leave.propagate(false);
                                let entity = leave.entity;

                                let hit = leave.hit.clone();
                                let pointer_id = leave.pointer_id;
                                let pointer_location = leave.pointer_location.clone();

                                if !disabled.contains(entity) {
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
                                }
                                cleanup_move_observer::<HoverMoveObserver>(&mut commands, entity, move_observer);
                            },
                        );
                    }),
                )
                .apply(remove_system_holder_on_despawn(hover_handler_holder))
                .apply(remove_system_holder_on_despawn(hovering_handler_holder))
        })
    }

    /// On frames where this element is hovered or gets unhovered, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HoverData`]. While hovered, the
    /// handler will be executed every frame. Whether hover handling is disabled is reactively
    /// controlled with a [`Signal`]. This method can be called repeatedly to register many such
    /// handlers.
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_hovered_disableable`] with a custom `Disabled` component and
    /// configure systems to run around [`hovered_system`].
    fn on_hovered_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_hovered_disableable::<HoverHandlingDisabled, _>(disableable_signal_system(system_holder, state_index))
    }

    /// On frames where this element is hovered or gets unhovered, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HoverData`]. This method can be called
    /// repeatedly to register many such handlers.
    fn on_hovered<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_hovered_disableable::<HoverHandlingDisabled, _>(handler)
    }

    /// When this element's hovered state changes and does not have a `Disabled` [`Component`], run
    /// a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`HoverData`].
    /// This method can be called repeatedly to register many such handlers.
    fn on_hovered_change_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(change_setup(handler, system_holder.clone()))
            .on_hovered_disableable::<Disabled, _>(change_system(system_holder, |d| d.hovered))
    }

    /// When this element's hovered state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HoverData`]. Whether hover change
    /// handling is disabled is reactively controlled with a [`Signal`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_hovered_change_disableable`] with a custom `Disabled`
    /// component and configure systems to run around [`hovered_system`].
    fn on_hovered_change_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_hovered_change_disableable::<HoverHandlingDisabled, _>(disableable_signal_system(
            system_holder,
            state_index,
        ))
    }

    /// When this element's hovered state changes, run a [`System`] which takes [`In`](`System::In`)
    /// this element's [`Entity`] and [`HoverData`]. This method can be called repeatedly to
    /// register many such handlers.
    fn on_hovered_change<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_hovered_change_disableable::<HoverHandlingDisabled, _>(handler)
    }

    /// On frames where this element is hovered or gets unhovered, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`HoverData`].
    ///
    /// The handler fires immediately when hover starts, then throttles by `duration` while
    /// hovered. When the pointer leaves and re-enters, it fires immediately again.
    fn on_hovered_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, HoverData)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let timer_id = Arc::new(OnceLock::new());
        self.with_builder(throttle_setup(
            handler,
            duration,
            system_holder.clone(),
            timer_id.clone(),
        ))
        .on_hovered_disableable::<HoverHandlingDisabled, _>(throttle_system(system_holder, timer_id, |d| d.hovered))
    }

    /// On frames where this element is clicked and does not have a `Disabled` [`Component`],
    /// run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and
    /// [`Pointer<Click>`]. This method can be called repeatedly to register many such handlers.
    ///
    /// Click event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Disabled;
    ///
    /// El::<Node>::new()
    ///     .on_click_disableable::<Disabled, _>(|In((_entity, _click))| {
    ///         // handle click
    ///     })
    ///     .observe(|mut click: On<Pointer<Click>>| {
    ///         click.propagate(false);
    ///     });
    /// ```
    fn on_click_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let system_holder = Arc::new(OnceLock::new());
            builder
                .on_spawn(clone!((system_holder) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    observe(world, entity, move |click: On<Pointer<Click>>, disabled: Query<&Disabled>, mut commands: Commands| {
                        if disabled.contains(click.entity) {
                            return;
                        }
                        commands.run_system_with(system, (click.entity, (*click).clone()));
                    });
                }))
                .apply(remove_system_holder_on_despawn(system_holder))
        })
    }

    /// Run a [`System`] when this element is clicked. This method can be called repeatedly to
    /// register many such handlers.
    ///
    /// Click event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_click(|In((_entity, _click))| {
    ///         // handle click
    ///     })
    ///     .observe(|mut click: On<Pointer<Click>>| {
    ///         click.propagate(false);
    ///     });
    /// ```
    fn on_click<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_click_disableable::<ClickHandlingDisabled, _>(handler)
    }

    /// Run a [`System`] when this element is clicked. Whether click handling is disabled is
    /// reactively controlled with a [`Signal`]. This method can be called repeatedly to register
    /// many such handlers.
    ///
    /// Click event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_click_disableable_signal(
    ///         |In(_)| {
    ///             // handle click
    ///         },
    ///         signal::once(false),
    ///     )
    ///     .observe(|mut click: On<Pointer<Click>>| {
    ///         click.propagate(false);
    ///     });
    /// ```
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_click_disableable`] with a custom `Disabled` component.
    fn on_click_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_click_disableable::<ClickHandlingDisabled, _>(disableable_signal_system(system_holder, state_index))
    }

    /// Run a [`System`] when this element is clicked, throttled by `duration` before the handler
    /// can run again.
    ///
    /// Click event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_click_throttled(|In((_entity, _click))| {
    ///         // handle click (throttled)
    ///     }, std::time::Duration::from_millis(100))
    ///     .observe(|mut click: On<Pointer<Click>>| {
    ///         click.propagate(false);
    ///     });
    /// ```
    fn on_click_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let timer_id = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .on_spawn(clone!((system_holder, timer_id) move |world, entity| {
                    let system = register_system(world, handler);
                    let _ = system_holder.set(system);
                    let _ = timer_id.set(add_throttle_timer_ready(world, entity, duration));
                    observe(world, entity, move |click: On<Pointer<Click>>,
                                                 time: Res<Time>,
                                                 mut collections: Query<&mut ThrottleTimers>,
                                                 mut commands: Commands| {
                        if let (Ok(mut collection), Some(&id)) = (collections.get_mut(click.entity), timer_id.get())
                            && collection.check_discrete(id, time.delta()) {
                                commands.run_system_with(system, (click.entity, (*click).clone()));
                            }
                    });
                }))
                .apply(remove_system_holder_on_despawn(system_holder))
        })
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents and the element does not have a `Disabled` [`Component`], run a
    /// [`System`] that takes [`In`](`System::In`) this element's [`Entity`] and the
    /// [`Pointer<Click>`]. Will not function unless this element is a descendant of a [`UiRoot`].
    /// This method can be called repeatedly to register many such handlers.
    fn on_click_outside_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, GlobalEventData<Pointer<Click>>)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .on_spawn(clone!((system_holder) move |world, _| {
                    let _ = system_holder.set(register_system(world, handler));
                }))
                .apply(remove_system_holder_on_despawn(system_holder.clone()))
        })
        .on_global_event::<Pointer<Click>, _, _, _>(
            move |In((entity, click)): In<(Entity, GlobalEventData<Pointer<Click>>)>,
                  childrens: Query<&Children>,
                  child_ofs: Query<&ChildOf>,
                  ui_roots: Query<&UiRoot>,
                  disabled: Query<&Disabled>,
                  mut commands: Commands| {
                if disabled.contains(entity) {
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
        self.on_click_outside_disableable::<ClickOutsideHandlingDisabled, _>(handler)
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a [`System`] that takes [`In`](`System::In`) this element's
    /// [`Entity`] and the [`Pointer<Click>`]. Will not function unless this element is a descendant
    /// of a [`UiRoot`]. Whether click outside handling is disabled is reactively controlled with a
    /// [`Signal`]. This method can be called repeatedly to register many such handlers.
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_click_outside_disableable`] with a custom `Disabled` component.
    fn on_click_outside_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, GlobalEventData<Pointer<Click>>)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_click_outside_disableable::<ClickOutsideHandlingDisabled, _>(disableable_signal_system(
            system_holder,
            state_index,
        ))
    }

    /// When a [`Pointer<Click>`] is received outside this [`Element`](super::element::Element)
    /// or its descendents, run a [`System`] that takes [`In`](`System::In`) this element's
    /// [`Entity`] and the [`Pointer<Click>`], throttled by `duration` before the handler can run
    /// again. Will not function unless this element is a descendant of a [`UiRoot`]. This method
    /// can be called repeatedly to register many such handlers.
    fn on_click_outside_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, GlobalEventData<Pointer<Click>>)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let timer_id = Arc::new(OnceLock::new());
        self.with_builder(|builder| {
            builder
                .on_spawn(clone!((system_holder, timer_id) move |world, entity| {
                    let _ = system_holder.set(register_system(world, handler));
                    let _ = timer_id.set(add_throttle_timer_ready(world, entity, duration));
                }))
                .apply(remove_system_holder_on_despawn(system_holder.clone()))
        })
        .on_global_event::<Pointer<Click>, _, _, _>(
            move |In((entity, click)): In<(Entity, GlobalEventData<Pointer<Click>>)>,
                  childrens: Query<&Children>,
                  child_ofs: Query<&ChildOf>,
                  ui_roots: Query<&UiRoot>,
                  time: Res<Time>,
                  mut collections: Query<&mut ThrottleTimers>,
                  mut commands: Commands| {
                for ancestor in child_ofs.iter_ancestors(entity) {
                    if ui_roots.contains(ancestor) {
                        if !is_inside_or_removed_from_dom(entity, &click, ancestor, &childrens)
                            && let (Ok(mut collection), Some(&id)) = (collections.get_mut(entity), timer_id.get())
                                && collection.check_discrete(id, time.delta()) {
                                    commands.run_system_with(system_holder.get().copied().unwrap(), (entity, click));
                                }
                        break;
                    }
                }
            },
        )
    }

    /// On frames where this element is pressed or gets unpressed and does not have a `Disabled`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and [`PressData`]. This method can be called repeatedly to register many such handlers.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed,
    /// use a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Disabled;
    ///
    /// El::<Node>::new()
    ///     .on_pressed_disableable::<Disabled, _>(|In((_entity, _press))| {
    ///         // handle press
    ///     })
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    fn on_pressed_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let press_handler_holder = Arc::new(OnceLock::new());
            let pressing_handler_holder = Arc::new(OnceLock::new());
            builder
                .insert(Pressable)
                .on_spawn(
                    clone!((press_handler_holder, pressing_handler_holder) move |world, entity| {
                        let press_handler_system = register_system(world, handler);
                        let _ = press_handler_holder.set(press_handler_system);

                        let pressing_handler_system = register_system(
                            world,
                            move |In(entity): In<Entity>,
                                  press_datas: Query<&PressDataInternal>,
                                  disabled: Query<&Disabled>,
                                  mut commands: Commands| {
                                if disabled.contains(entity) {
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
                                  disabled: Query<&Disabled>,
                                  move_observers: Query<&PressMoveObserver>,
                                  mut commands: Commands| {
                                let entity = press.entity;
                                if disabled.contains(entity) {
                                    return;
                                }

                                let button = press.button;
                                let hit = press.hit.clone();
                                let pointer_id = press.pointer_id;
                                let pointer_location = press.pointer_location.clone();

                                let move_observer = (!move_observers.contains(entity)).then(|| {
                                    create_move_observer::<PressDataInternal>(&mut commands, entity)
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

                                commands.run_system_with(
                                    press_handler_system,
                                    (entity, PressData {
                                        pressed: true,
                                        button,
                                        hit,
                                        pointer_id,
                                        pointer_location,
                                    }),
                                );
                            },
                        );

                        observe(
                            world,
                            entity,
                                move |release: On<Pointer<Release>>,
                                    disabled: Query<&Disabled>,
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

                                if !disabled.contains(entity) {
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
                                }
                                cleanup_move_observer::<PressMoveObserver>(&mut commands, entity, move_observer);
                            },
                        );
                    }),
                )
                .apply(remove_system_holder_on_despawn(press_handler_holder))
                .apply(remove_system_holder_on_despawn(pressing_handler_holder))
        })
        .on_hovered_change(
            |In((entity, data)): In<(Entity, HoverData)>,
             move_observers: Query<&PressMoveObserver>,
             mut commands: Commands| {
                if !data.hovered {
                    let move_observer = move_observers.get(entity).ok().map(|o| o.0);
                    if let Ok(mut entity) = commands.get_entity(entity) {
                        entity.remove::<PressDataInternal>();
                        entity.remove::<PressedSystem>();
                    }
                    cleanup_move_observer::<PressMoveObserver>(&mut commands, entity, move_observer);
                }
            },
        )
    }

    /// On frames where this element is pressed or gets unpressed, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`PressData`]. Whether press handling
    /// is disabled is reactively controlled with a [`Signal`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_pressed_disableable_signal(
    ///         |In(_)| {
    ///             // handle press
    ///         },
    ///         signal::once(false),
    ///     )
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_pressed_disableable`] with a custom `Disabled` component and
    /// configure systems to run around [`pressed_system`].
    fn on_pressed_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_pressed_disableable::<PressHandlingDisabled, _>(disableable_signal_system(system_holder, state_index))
    }

    /// On frames where this element is pressed or gets unpressed, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`PressData`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed,
    /// use a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_pressed(|In((_entity, _press))| {
    ///         // handle press
    ///     })
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    fn on_pressed<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_pressed_disableable::<PressHandlingDisabled, _>(handler)
    }

    /// When this element's pressed state changes and does not have a `Disabled` [`Component`], run
    /// a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`PressData`].
    /// This method can be called repeatedly to register many such handlers.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Disabled;
    ///
    /// El::<Node>::new()
    ///     .on_pressed_change_disableable::<Disabled, _>(|In((_entity, _press))| {
    ///         // handle pressed state changes
    ///     })
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    fn on_pressed_change_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(change_setup(handler, system_holder.clone()))
            .on_pressed_disableable::<Disabled, _>(change_system(system_holder, |d| d.pressed))
    }

    /// When this element's pressed state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`PressData`]. Whether press change
    /// handling is disabled is reactively controlled with a [`Signal`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_pressed_change_disableable_signal(
    ///         |In((_entity, _press))| {
    ///             // handle pressed state changes
    ///         },
    ///         signal::once(false),
    ///     )
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_pressed_change_disableable`] with a custom `Disabled`
    /// component and configure systems to run around [`pressed_system`].
    fn on_pressed_change_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_pressed_change_disableable::<PressHandlingDisabled, _>(disableable_signal_system(
            system_holder,
            state_index,
        ))
    }

    /// When this element's pressed state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`PressData`]. This method can be
    /// called repeatedly to register many such handlers.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed,
    /// use a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_pressed_change(|In((_entity, _press))| {
    ///         // handle pressed state changes
    ///     })
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    fn on_pressed_change<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_pressed_change_disableable::<PressHandlingDisabled, _>(handler)
    }

    /// On frames where this element is pressed or gets unpressed, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`PressData`].
    ///
    /// The handler fires immediately when pressed, then throttles by `duration` while held.
    /// When released and pressed again, it fires immediately again.
    ///
    /// Press event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_pressed_throttled(|In((_entity, _press))| {
    ///         // handle pressing (throttled)
    ///     }, std::time::Duration::from_millis(100))
    ///     .observe(|mut press: On<Pointer<Press>>| {
    ///         press.propagate(false);
    ///     });
    /// ```
    fn on_pressed_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, PressData)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let timer_id = Arc::new(OnceLock::new());
        self.with_builder(throttle_setup(
            handler,
            duration,
            system_holder.clone(),
            timer_id.clone(),
        ))
        .on_pressed_disableable::<PressHandlingDisabled, _>(throttle_system(system_holder, timer_id, |d| d.pressed))
    }

    /// On frames where this element is dragged or gets undragged and does not have a `Disabled`
    /// [`Component`], run a [`System`] which takes [`In`](`System::In`) this element's [`Entity`]
    /// and [`DragData`]. This method can be called repeatedly to register many such handlers.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed,
    /// use a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Disabled;
    ///
    /// El::<Node>::new()
    ///     .on_dragged_disableable::<Disabled, _>(|In((_entity, _drag))| {
    ///         // handle drag start/end
    ///     })
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    fn on_dragged_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            let drag_handler_holder = Arc::new(OnceLock::new());
            let dragging_handler_holder = Arc::new(OnceLock::new());
            builder
                .insert(Draggable)
                .on_spawn(
                    clone!((drag_handler_holder, dragging_handler_holder) move |world, entity| {
                        let drag_handler_system = register_system(world, handler);
                        let _ = drag_handler_holder.set(drag_handler_system);

                        let dragging_handler_system = register_system(
                            world,
                            move |In(entity): In<Entity>,
                                  disabled: Query<&Disabled>,
                                  mut drag_datas: Query<&mut DragDataInternal>,
                                  mut commands: Commands| {
                                if disabled.contains(entity) {
                                    return;
                                }
                                if let Ok(mut drag_data) = drag_datas.get_mut(entity) {
                                    if !drag_data.has_new_delta {
                                        return;
                                    }
                                    drag_data.has_new_delta = false;
                                    // Extract accumulated delta and reset for next frame
                                    let delta = drag_data.delta;
                                    drag_data.delta = Vec2::ZERO;
                                    commands.run_system_with(
                                        drag_handler_system,
                                        (
                                            entity,
                                            DragData {
                                                dragged: true,
                                                button: drag_data.button,
                                                pointer_id: drag_data.pointer_id,
                                                pointer_location: drag_data.pointer_location.clone(),
                                                hit: drag_data.hit.clone(),
                                                delta,
                                            },
                                        ),
                                    );
                                }
                            },
                        );
                        let _ = dragging_handler_holder.set(dragging_handler_system);

                        observe(
                            world,
                            entity,
                            move |drag_start: On<Pointer<DragStart>>,
                                  disabled: Query<&Disabled>,
                                  drag_observers: Query<&DragMoveObserver>,
                                  dragged_systems: Query<&DraggedSystem>,
                                  mut commands: Commands| {
                                let entity = drag_start.entity;
                                if disabled.contains(entity) {
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
                                                        && drag_data.button == drag_event.button
                                                    {
                                                        drag_data.pointer_location =
                                                            drag_event.pointer_location.clone();
                                                        // Accumulate deltas to handle multiple events per frame
                                                        drag_data.delta += drag_event.delta;
                                                        drag_data.has_new_delta = true;
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
                                        has_new_delta: false,
                                    });
                                    entity.insert(Dragged);
                                    if let Some(drag_observer) = drag_observer {
                                        entity.insert(DragMoveObserver(drag_observer));
                                    }
                                    if !dragged_systems.contains(entity.id()) {
                                        entity.insert(DraggedSystem(dragging_handler_system));
                                    }
                                }

                                commands.run_system_with(
                                    drag_handler_system,
                                    (entity, DragData {
                                        dragged: true,
                                        button,
                                        hit,
                                        pointer_id,
                                        pointer_location,
                                        delta: Vec2::ZERO,
                                    }),
                                );
                            },
                        );

                        observe(
                            world,
                            entity,
                            move |drag_end: On<Pointer<DragEnd>>,
                                disabled: Query<&Disabled>,
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

                                if !disabled.contains(entity) {
                                    commands.run_system_with(drag_handler_system, (entity, DragData {
                                        dragged: false,
                                        button,
                                        hit: stored_hit,
                                        pointer_id,
                                        pointer_location,
                                        delta: Vec2::ZERO,
                                    }));
                                }

                                if let Ok(mut entity) = commands.get_entity(entity) {
                                    entity.remove::<DragDataInternal>();
                                    entity.remove::<Dragged>();
                                    entity.remove::<DraggedSystem>();
                                }
                                cleanup_move_observer::<DragMoveObserver>(&mut commands, entity, drag_observer);
                            },
                        );
                    }),
                )
                .apply(remove_system_holder_on_despawn(drag_handler_holder))
                .apply(remove_system_holder_on_despawn(dragging_handler_holder))
        })
    }

    /// On frames where this element is dragged or gets undragged, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`DragData`]. Whether drag handling
    /// is disabled is reactively controlled with a [`Signal`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_dragged_disableable_signal(
    ///         |In(_)| {
    ///             // handle drag start/end
    ///         },
    ///         signal::once(false),
    ///     )
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_dragged_disableable`] with a custom `Disabled` component and
    /// configure systems to run around [`dragged_system`].
    fn on_dragged_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_dragged_disableable::<DragHandlingDisabled, _>(disableable_signal_system(system_holder, state_index))
    }

    /// On frames where this element is dragged or gets undragged, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`DragData`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed,
    /// use a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_dragged(|In((_entity, _drag))| {
    ///         // handle drag
    ///     })
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    fn on_dragged<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_dragged_disableable::<DragHandlingDisabled, _>(handler)
    }

    /// When this element's dragged state changes and does not have a `Disabled` [`Component`], run
    /// a [`System`] which takes [`In`](`System::In`) this element's [`Entity`] and [`DragData`].
    /// This method can be called repeatedly to register many such handlers.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Disabled;
    ///
    /// El::<Node>::new()
    ///     .on_dragged_change_disableable::<Disabled, _>(|In((_entity, _drag))| {
    ///         // handle dragged state changes
    ///     })
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    fn on_dragged_change_disableable<Disabled: Component, Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self.with_builder(change_setup(handler, system_holder.clone()))
            .on_dragged_disableable::<Disabled, _>(change_system(system_holder, |d| d.dragged))
    }

    /// When this element's dragged state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`DragData`]. Whether drag change
    /// handling is disabled is reactively controlled with a [`Signal`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_dragged_change_disableable_signal(
    ///         |In((_entity, _drag))| {
    ///             // handle dragged state changes
    ///         },
    ///         signal::once(false),
    ///     )
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    ///
    /// # Latency Considerations
    ///
    /// Because [haalka](crate) runs [jonmo]'s [`SignalProcessing`](jonmo::SignalProcessing) system
    /// set in [`PostUpdate`] (unless configured otherwise), there may be
    /// a 1-frame delay between the signal outputting a new disabled state and the handler
    /// reflecting that state. For synchronous control over disabling, use
    /// [`PointerEventAware::on_dragged_change_disableable`] with a custom `Disabled`
    /// component and configure systems to run around [`dragged_system`].
    fn on_dragged_change_disableable_signal<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let state_index = Arc::new(OnceLock::new());
        self.with_builder(disableable_signal_setup(
            handler,
            disabled,
            system_holder.clone(),
            state_index.clone(),
        ))
        .on_dragged_change_disableable::<DragHandlingDisabled, _>(disableable_signal_system(system_holder, state_index))
    }

    /// When this element's dragged state changes, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`DragData`]. This method can be called
    /// repeatedly to register many such handlers.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed,
    /// use a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_dragged_change(|In((_entity, _drag))| {
    ///         // handle dragged state changes
    ///     })
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    fn on_dragged_change<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
    ) -> Self {
        self.on_dragged_change_disableable::<DragHandlingDisabled, _>(handler)
    }

    /// On frames where this element is dragged or gets undragged, run a [`System`] which takes
    /// [`In`](`System::In`) this element's [`Entity`] and [`DragData`].
    ///
    /// The handler fires immediately when dragging starts, then throttles by `duration` while
    /// dragged. When the drag ends and starts again, it fires immediately again.
    ///
    /// Drag event propagation cannot be stopped through this method (yet); if this is needed, use
    /// a separate observer, e.g.
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// El::<Node>::new()
    ///     .on_dragged_throttled(|In((_entity, _drag))| {
    ///         // handle dragging (throttled)
    ///     }, std::time::Duration::from_millis(100))
    ///     .observe(|mut drag_start: On<Pointer<DragStart>>| {
    ///         drag_start.propagate(false);
    ///     });
    /// ```
    fn on_dragged_throttled<Marker>(
        self,
        handler: impl IntoSystem<In<(Entity, DragData)>, (), Marker> + Send + Sync + 'static,
        duration: Duration,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        let timer_id = Arc::new(OnceLock::new());
        self.with_builder(throttle_setup(
            handler,
            duration,
            system_holder.clone(),
            timer_id.clone(),
        ))
        .on_dragged_disableable::<DragHandlingDisabled, _>(throttle_system(system_holder, timer_id, |d| d.dragged))
    }
}

#[derive(Component, Clone)]
/// Marker component for entities currently hovered by a pointer.
pub struct Hovered;

#[derive(Component, Clone)]
/// Marker component for entities currently being dragged.
pub struct Dragged;

#[derive(Component, Default, Clone)]
struct PressHandlingDisabled;

#[derive(Component, Default, Clone)]
struct HoverHandlingDisabled;

#[derive(Component, Default, Clone)]
struct DragHandlingDisabled;

#[derive(Component, Default, Clone)]
struct ClickHandlingDisabled;

#[derive(Component, Default, Clone)]
struct ClickOutsideHandlingDisabled;

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

/// Caches the most recent `HitData` observed anywhere in this entity's hovered subtree.
///
/// This enables emitting `Leave { hit: .. }` under subtree-hover semantics even though the
/// picking `HoverMap` / `PreviousHoverMap` typically only includes directly hovered entities.
#[derive(Component, Clone)]
struct LastSubtreeHoverHit(HitData);

#[allow(clippy::type_complexity)]
fn update_hover_states(
    pointer_map: Res<PointerMap>,
    pointers: Query<&PointerLocation>,
    hover_map: Res<HoverMap>,
    mut hovereds: Query<(Entity, Option<&Hovered>, Option<&LastSubtreeHoverHit>), Or<(With<Hoverable>, With<Hovered>)>>,
    child_ofs: Query<&ChildOf>,
    mut commands: Commands,
) {
    let pointer_id = PointerId::Mouse;
    let hover_set = hover_map.get(&pointer_id);

    for (entity, hovered, last_subtree_hit) in hovereds.iter_mut() {
        let hit_data_option = hover_set.and_then(|map| {
            if let Some(hit) = map.get(&entity) {
                Some(hit)
            } else {
                map.iter()
                    .find(|(hit_entity, _)| child_ofs.iter_ancestors(**hit_entity).any(|e| e == entity))
                    .map(|(_, hit_data)| hit_data)
            }
        });

        // Keep the cached hit fresh while hovered (self or descendant).
        if let Some(hit) = hit_data_option {
            let should_update_cache = last_subtree_hit.is_none_or(|cached| cached.0 != *hit);
            if should_update_cache && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_insert(LastSubtreeHoverHit(hit.clone()));
            }
        }

        // Semantics:
        // - Entered when self OR any descendant is hovered.
        // - Left when self AND all descendants are not hovered.
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
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.try_insert(Hovered);
                }
                continue;
            }

            if let Some(hit) = last_subtree_hit.map(|h| h.0.clone()) {
                commands.trigger(Pointer::new(pointer_id, location, Leave { hit }, entity));
            } else {
                debug!(
                    "Unable to get cached subtree hit for pointer {:?} leave on {:?}",
                    pointer_id, entity
                );
            }

            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.remove::<Hovered>();
                entity_commands.remove::<LastSubtreeHoverHit>();
            }
        }
    }
}

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

/// Marker component that enables hover state management for an entity.
///
/// When this component is present on an entity, the [`Hovered`] component will be
/// automatically inserted when the entity (or any of its descendants) is hovered,
/// and removed when it's no longer hovered. This allows using [`Hovered`] in queries
/// and signals without needing to use the [`PointerEventAware::on_hovered`] methods.
#[derive(Component)]
pub struct Hoverable;

/// Marker component that enables press state management for an entity.
///
/// When this component is present on an entity, the [`Pressed`] component
/// will be automatically inserted when the entity is pressed, and removed when released.
/// This allows using `Pressed` in queries and signals without needing to use the
/// [`PointerEventAware::on_pressed`] methods.
#[derive(Component)]
pub struct Pressable;

/// Marker component that enables drag state management for an entity.
///
/// When this component is present on an entity, the [`Dragged`] component will be
/// automatically inserted when the entity starts being dragged, and removed when
/// dragging ends. This allows using [`Dragged`] in queries and signals without
/// needing to use the [`PointerEventAware::on_dragged`] methods.
#[derive(Component)]
#[component(on_add = on_draggable_add, on_remove = on_draggable_remove)]
pub struct Draggable;

/// Stores the observer entities created for a [`Draggable`] component.
#[derive(Component)]
struct DraggableObservers {
    drag_start: Entity,
    drag_end: Entity,
}

fn on_draggable_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world.commands().queue(move |world: &mut World| {
        let drag_start = world
            .entity_mut(entity)
            .observe(|drag_start: On<Pointer<DragStart>>, mut commands: Commands| {
                if let Ok(mut entity) = commands.get_entity(drag_start.entity) {
                    entity.insert(Dragged);
                }
            })
            .id();
        let drag_end = world
            .entity_mut(entity)
            .observe(|drag_end: On<Pointer<DragEnd>>, mut commands: Commands| {
                if let Ok(mut entity) = commands.get_entity(drag_end.entity) {
                    entity.remove::<Dragged>();
                }
            })
            .id();
        world
            .entity_mut(entity)
            .insert(DraggableObservers { drag_start, drag_end });
    });
}

fn on_draggable_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Some(observers) = world.get::<DraggableObservers>(entity) {
        let drag_start = observers.drag_start;
        let drag_end = observers.drag_end;
        world.commands().queue(move |world: &mut World| {
            let _ = world.try_despawn(drag_start);
            let _ = world.try_despawn(drag_end);
        });
    }
    world.commands().queue(move |world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.remove::<DraggableObservers>();
            entity.remove::<Dragged>();
        }
    });
}

#[derive(Component)]
struct PressedSystem(SystemId<In<Entity>, ()>);

#[derive(Component, Clone)]
struct DragDataInternal {
    button: PointerButton,
    hit: HitData,
    pointer_id: PointerId,
    pointer_location: Location,
    delta: Vec2,
    has_new_delta: bool,
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

/// System that runs registered press handlers for pressed entities.
///
/// This system is exposed publicly so users can order their own systems around it
/// when using [`PointerEventAware::on_pressed_disableable`] with custom `Disabled` components.
/// Runs in [`Update`].
#[allow(private_interfaces)]
pub fn pressed_system(mut interaction_query: Query<(Entity, &PressedSystem), With<Pressed>>, mut commands: Commands) {
    for (entity, &PressedSystem(system)) in &mut interaction_query {
        commands.run_system_with(system, entity);
    }
}

/// System that runs registered drag handlers for dragged entities.
///
/// This system is exposed publicly so users can order their own systems around it
/// when using [`PointerEventAware::on_dragged_disableable`] with custom `Disabled` components.
/// Runs in [`Update`].
#[allow(private_interfaces)]
pub fn dragged_system(mut interaction_query: Query<(Entity, &DraggedSystem), With<Dragged>>, mut commands: Commands) {
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

/// System that runs registered hover handlers for hovered entities.
///
/// This system is exposed publicly so users can order their own systems around it
/// when using [`PointerEventAware::on_hovered_disableable`] with custom `Disabled` components.
/// Runs in [`Update`].
#[allow(private_interfaces)]
pub fn hovered_system(mut hovering_query: Query<(Entity, &HoveredSystem), With<Hovered>>, mut commands: Commands) {
    for (entity, &HoveredSystem(system)) in &mut hovering_query {
        commands.run_system_with(system, entity);
    }
}

fn cleanup_move_observer<T: Component>(commands: &mut Commands, entity: Entity, observer: Option<Entity>) {
    if let Ok(mut entity_commands) = commands.get_entity(entity)
        && observer.is_some() {
            entity_commands.remove::<T>();
        }
    if let Some(observer_entity) = observer {
        commands.entity(observer_entity).despawn();
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

/// When this [`Resource`] exists in the [`World`], [`Cursorable`]
/// [`Element`]s will not trigger updates to the window's cursor when they
/// receive a [`Pointer<Over>`] event. When this [`Resource`] is removed, the last
/// [`Option<CursorIcon>`] queued by a [`Cursorable`] [`Element`] will be set as the window's
/// cursor. Adding this [`Resource`] to the [`World`] will *not* unset any [`Option<CursorIcon>`]s
/// previously set by a [`Cursorable`] [`Element`].
///
/// [`Element`]: super::element::Element
#[derive(Resource)]
pub struct CursorableDisabled;

/// A [`Component`] which stores the [`Option<CursorIcon>`] to set the window's cursor to when an
/// [`Element`](super::element::Element) receives a [`Pointer<Over>`] event; when [`None`], the
/// cursor will be hidden.
#[derive(Component, Clone)]
pub struct Cursor(Option<CursorIcon>);

/// Enables managing the window's [`CursorIcon`] when a [`Pickable`]
/// [`Element`](super::element::Element) receives an [`Pointer<Over>`] event.
///
/// **Note:** These methods will only function for [`Pickable`] entities (e.g., those with
/// [`Pickable::default()`]).
pub trait Cursorable: PointerEventAware {
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
                     cursor_on_hovers: Query<&Cursor>,
                     disabled: Query<&Disabled>,
                     cursor_over_disabled_option: Option<Res<CursorableDisabled>>,
                     mut commands: Commands| {
                        let entity = event.entity;
                        if let Ok(Cursor(cursor_option)) = cursor_on_hovers.get(entity).cloned() {
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
                    |event: On<Insert, Cursor>, cursor_overs: Query<&CursorOver>, mut commands: Commands| {
                        let entity = event.entity;
                        if cursor_overs.contains(entity)
                            && let Ok(mut entity) = commands.get_entity(entity)
                        {
                            entity.try_insert(CursorOver);
                        }
                    },
                )
                .insert(Cursor(cursor_option))
                .observe(
                    move |event: On<Insert, Disabled>,
                          cursor_over: Query<&CursorOver>,
                          pointer_map: Res<PointerMap>,
                          pointers: Query<&PointerLocation>,
                          hover_map: Res<HoverMap>,
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
                            commands.trigger(Pointer::new(PointerId::Mouse, location, Over { hit }, parent));
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
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + 'static,
    ) -> Self {
        self.with_builder(|builder| {
            builder.component_signal::<Cursor>(
                cursor_option_signal.map_in(|into_option_cursor| Some(Cursor(into_option_cursor.into()))),
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
        cursor_option_signal: impl Signal<Item = impl Into<Option<CursorIcon>> + 'static> + 'static,

        disabled: impl Signal<Item = bool> + 'static,
    ) -> Self {
        self.with_builder(|builder| builder.component_signal(disabled.map_true_in(|| CursorDisabled)))
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
        disabled: impl Signal<Item = bool> + 'static,

    ) -> Self {
        self.cursor_signal_disableable_signal(signal::once(cursor_option.into()), disabled)
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
        PreUpdate,
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
            consume_queued_cursor.run_if(resource_removed::<CursorableDisabled>),
        )
            .after(PickingSystems::Last),
    );
}
