//! Low level reactive entity management ported from [Dominator](https://github.com/Pauan/rust-dominator)'s [`DomBuilder`](https://docs.rs/dominator/latest/dominator/struct.DomBuilder.html).

use std::sync::OnceLock;

use apply::Apply;
use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};
use bevy_async_ecs::AsyncWorld;
use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{MutableVec, SignalVec, SignalVecExt, VecDiff},
};
use haalka_futures_signals_ext::{Future, MutableExt};
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use haalka_futures_signals_ext::future::AbortHandle;
    } else {
        use bevy::tasks::Task;
    }
}

// TODO: 0.15 `Task` api is unified, can remove
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        pub struct WasmTaskAdapter(pub AbortHandle);

        impl WasmTaskAdapter {
            pub fn detach(self) {}
        }
    }
}

use super::utils::{clone, spawn};

static ASYNC_WORLD: OnceLock<AsyncWorld> = OnceLock::new();

/// Global access to [`bevy_async_ecs::AsyncWorld`], providing convenient access to the [`World`]
/// from deeply nested async contexts.
pub fn async_world() -> &'static AsyncWorld {
    ASYNC_WORLD.get().expect("expected ASYNC_WORLD to be initialized")
}

pub(crate) fn init_async_world(world: &mut World) {
    ASYNC_WORLD
        .set(AsyncWorld::from_world(world))
        .expect("failed to initialize ASYNC_WORLD");
}

/// A thin facade over a Bevy [`Entity`] enabling the ergonomic registration of reactive tasks and
/// children using a declarative builder pattern/[fluent interface](https://en.wikipedia.org/wiki/Fluent_interface).
/// Port of [Dominator](https://github.com/Pauan/rust-dominator)'s [`DomBuilder`](https://docs.rs/dominator/latest/dominator/struct.DomBuilder.html).
#[derive(Default)]
pub struct NodeBuilder {
    on_spawns: Vec<Box<dyn FnOnce(&mut World, Entity) + Send>>,
    // TODO: 0.15 `Task` api is unified, can remove branching
    #[cfg(target_arch = "wasm32")]
    task_wrappers: Vec<Box<dyn FnOnce(Entity) -> WasmTaskAdapter + Send>>,
    #[cfg(not(target_arch = "wasm32"))]
    task_wrappers: Vec<Box<dyn FnOnce(Entity) -> Task<()> + Send>>,
    child_block_populations: MutableVec<usize>,
}

impl<T: Bundle> From<T> for NodeBuilder {
    fn from(bundle: T) -> Self {
        default::<NodeBuilder>().insert(bundle)
    }
}

impl NodeBuilder {
    /// Run a function with mutable access to the [`World`] and this node's [`Entity`].
    pub fn on_spawn(mut self, on_spawn: impl FnOnce(&mut World, Entity) + Send + 'static) -> Self {
        self.on_spawns.push(Box::new(on_spawn));
        self
    }

    /// Run a function with this node's [`EntityWorldMut`].
    pub fn with_entity(self, f: impl FnOnce(EntityWorldMut) + Send + 'static) -> Self {
        self.on_spawn(move |world, entity| {
            if let Some(entity) = world.get_entity_mut(entity) {
                f(entity);
            }
        })
    }

    /// Add a [`Bundle`] of components to the node.
    pub fn insert<B: Bundle>(self, bundle: B) -> Self {
        self.with_entity(|mut entity| {
            entity.insert(bundle);
        })
    }

    /// Reactively run a [`Future`]-returning function with this node's [`Entity`] and the output of
    /// the [`Signal`].
    pub fn on_signal<T, Fut: Future<Output = ()> + Send + 'static>(
        mut self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Entity, T) -> Fut + Send + 'static,
    ) -> Self {
        self.task_wrappers.push(Box::new(move |entity: Entity| {
            signal.for_each(move |value| f(entity, value)).apply(spawn)
        }));
        self
    }

    // TODO: list out limitations; limitation: if multiple children are added to entity, they must
    // be registered thru this abstraction because of the way siblings are tracked
    /// Declare a static child.
    pub fn child(self, child: NodeBuilder) -> Self {
        let block = self.child_block_populations.lock_ref().len();
        self.child_block_populations.lock_mut().push(1);
        let offset = offset(block, &self.child_block_populations.lock_ref());
        let on_spawn = move |world: &mut World, parent| {
            let child_entity = world.spawn_empty().id();
            if let Some(mut parent) = world.get_entity_mut(parent) {
                parent.insert_children(offset, &[child_entity]);
                child.spawn_on_entity(world, child_entity);
            } else {
                // parent despawned during child spawning
                if let Some(child) = world.get_entity_mut(child_entity) {
                    child.despawn_recursive();
                }
            }
        };
        self.on_spawn(on_spawn)
    }

    /// Declare a reactive child. When the [`Signal`] outputs [`None`], the child is removed.
    pub fn child_signal(
        mut self,
        child_option: impl Signal<Item = impl Into<Option<NodeBuilder>> + Send> + Send + 'static,
    ) -> Self {
        let block = self.child_block_populations.lock_ref().len();
        self.child_block_populations.lock_mut().push(0);
        let child_block_populations = self.child_block_populations.clone();
        let task_wrapper = move |entity: Entity| {
            let existing_child_option = Mutable::new(None);
            clone!((entity => parent) async move {
                child_option.for_each(move |child_option| {
                    clone!((existing_child_option, child_block_populations) async move {
                        if let Some(child) = child_option.into() {
                            async_world().apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();  // removes from parent
                                    }
                                }
                                let child_entity = world.spawn_empty().id();
                                if let Some(mut parent) = world.get_entity_mut(parent) {
                                    let offset = offset(block, &child_block_populations.lock_ref());
                                    parent.insert_children(offset, &[child_entity]);
                                    child.spawn_on_entity(world, child_entity);
                                    existing_child_option.set(Some(child_entity));
                                } else {  // parent despawned during child spawning
                                    if let Some(child) = world.get_entity_mut(child_entity) {
                                        child.despawn_recursive();
                                    }
                                }
                                child_block_populations.lock_mut().set(block, 1);
                            }).await;
                        } else {
                            async_world().apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();
                                    }
                                }
                                child_block_populations.lock_mut().set(block, 0);
                            })
                            .await;
                        }
                    })
                }).await;
            })
            .apply(spawn)
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    /// Declare static children.
    pub fn children(self, children: impl IntoIterator<Item = NodeBuilder> + Send + 'static) -> Self {
        let block = self.child_block_populations.lock_ref().len();
        let children = children.into_iter().collect::<Vec<_>>();
        let population = children.len();
        self.child_block_populations.lock_mut().push(population);
        let child_block_populations = self.child_block_populations.clone();
        let offset = offset(block, &child_block_populations.lock_ref());
        let on_spawn = move |world: &mut World, parent: Entity| {
            let mut children_entities = vec![];
            for _ in 0..children.len() {
                children_entities.push(world.spawn_empty().id());
            }
            if let Some(mut parent) = world.get_entity_mut(parent) {
                parent.insert_children(offset, &children_entities);
                for (child, child_entity) in children.into_iter().zip(children_entities) {
                    child.spawn_on_entity(world, child_entity);
                }
            } else {
                // parent despawned during child spawning
                for child in children_entities {
                    if let Some(child) = world.get_entity_mut(child) {
                        child.despawn_recursive();
                    }
                }
            }
        };
        self.on_spawn(on_spawn)
    }

    /// Declare reactive children.
    pub fn children_signal_vec(
        mut self,
        children_signal_vec: impl SignalVec<Item = NodeBuilder> + Send + 'static,
    ) -> Self {
        let block = self.child_block_populations.lock_ref().len();
        self.child_block_populations.lock_mut().push(0);
        let child_block_populations = self.child_block_populations.clone();
        let task_wrapper = move |entity: Entity| {
            clone!((entity => parent) {
                let children_entities = MutableVec::default();
                children_signal_vec
                .for_each(clone!((parent, children_entities, child_block_populations) move |diff| {
                    clone!((parent, children_entities, child_block_populations) async move {
                        // TODO: unit tests for every branch
                        match diff {
                            VecDiff::Replace { values: children } => {
                                async_world().apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    for child in children_lock.drain(..) {
                                        if let Some(child) = world.get_entity_mut(child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                    }
                                    for _ in 0..children.len() {
                                        children_lock.push(world.spawn_empty().id());
                                    }
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let offset = offset(block, &child_block_populations.lock_ref());
                                        parent.insert_children(offset, children_lock.as_slice());
                                        for (child, child_entity) in children.into_iter().zip(children_lock.iter().copied()) {
                                            child.spawn_on_entity(world, child_entity);
                                        }
                                        child_block_populations.lock_mut().set(block, children_lock.len());
                                    } else {  // parent despawned during child spawning
                                        for entity in children_lock.drain(..) {
                                            if let Some(child) = world.get_entity_mut(entity) {
                                                child.despawn_recursive();
                                            }
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::InsertAt { index, value: child } => {
                                async_world().apply(move |world: &mut World| {
                                    let child_entity = world.spawn_empty().id();
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let offset = offset(block, &child_block_populations.lock_ref());
                                        parent.insert_children(offset + index, &[child_entity]);
                                        child.spawn_on_entity(world, child_entity);
                                        let mut children_lock = children_entities.lock_mut();
                                        children_lock.insert(index, child_entity);
                                        child_block_populations.lock_mut().set(block, children_lock.len());
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::Push { value: child } => {
                                async_world().apply(move |world: &mut World| {
                                    let child_entity = world.spawn_empty().id();
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let mut children_lock = children_entities.lock_mut();
                                        let offset = offset(block, &child_block_populations.lock_ref());
                                        parent.insert_children(offset + children_lock.len(), &[child_entity]);
                                        child.spawn_on_entity(world, child_entity);
                                        children_lock.push(child_entity);
                                        child_block_populations.lock_mut().set(block, children_lock.len());
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::UpdateAt { index, value: node } => {
                                async_world().apply(move |world: &mut World| {
                                    if let Some(existing_child) = children_entities.lock_ref().get(index).copied() {
                                        if let Some(child) = world.get_entity_mut(existing_child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                    }
                                    let child_entity = world.spawn_empty().id();
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        children_entities.lock_mut().set(index, child_entity);
                                        let offset = offset(block, &child_block_populations.lock_ref());
                                        parent.insert_children(offset + index, &[child_entity]);
                                        node.spawn_on_entity(world, child_entity);
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::Move { old_index, new_index } => {
                                async_world().apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    children_lock.swap(old_index, new_index);
                                    // porting the swap implementation above
                                    fn move_from_to(parent: &mut EntityWorldMut, children_entities: &[Entity], old_index: usize, new_index: usize) {
                                        if old_index != new_index {
                                            if let Some(old_entity) = children_entities.get(old_index).copied() {
                                                parent.remove_children(&[old_entity]);
                                                parent.insert_children(new_index, &[old_entity]);
                                            }
                                        }
                                    }
                                    fn swap(parent: &mut EntityWorldMut, children_entities: &[Entity], a: usize, b: usize) {
                                        move_from_to(parent, children_entities, a, b);
                                        match a.cmp(&b) {
                                            std::cmp::Ordering::Less => {
                                                move_from_to(parent, children_entities, b - 1, a);
                                            }
                                            std::cmp::Ordering::Greater => {
                                                move_from_to(parent, children_entities, b + 1, a)
                                            }
                                            _ => {}
                                        }
                                    }
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let offset = offset(block, &child_block_populations.lock_ref());
                                        swap(&mut parent, children_lock.as_slice(), offset + old_index, offset + new_index);
                                    }
                                })
                                .await;
                            }
                            VecDiff::RemoveAt { index } => {
                                async_world().apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    if let Some(existing_child) = children_lock.get(index).copied() {
                                        if let Some(child) = world.get_entity_mut(existing_child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                        children_lock.remove(index);
                                        child_block_populations.lock_mut().set(block, children_lock.len());
                                    }
                                })
                                .await;
                            }
                            VecDiff::Pop {} => {
                                async_world().apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    if let Some(child_entity) = children_lock.pop() {
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                        child_block_populations.lock_mut().set(block, children_lock.len());
                                    }
                                })
                                .await;
                            }
                            VecDiff::Clear {} => {
                                async_world().apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    for child_entity in children_lock.drain(..) {
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                    child_block_populations.lock_mut().set(block, children_lock.len());
                                })
                                .await;
                            }
                        }
                    })
                }))
            })
            .apply(spawn)
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    /// Spawn a node on an existing [`Entity`].
    pub fn spawn_on_entity(self, world: &mut World, entity: Entity) {
        if let Some(mut entity) = world.get_entity_mut(entity) {
            let id = entity.id();
            entity.insert(TaskHolder::new());
            for on_spawn in self.on_spawns {
                on_spawn(world, id);
            }
            if !self.task_wrappers.is_empty() {
                if let Some(mut entity) = world.get_entity_mut(id) {
                    if let Some(mut task_holder) = entity.get_mut::<TaskHolder>() {
                        for task_wrapper in self.task_wrappers {
                            task_holder.hold(task_wrapper(id));
                        }
                    }
                }
            }
        }
    }

    /// Spawn a node into the world.
    pub fn spawn(self, world: &mut World) -> Entity {
        let id = world.spawn_empty().id();
        self.spawn_on_entity(world, id);
        id
    }
}

// TODO: tasks that resolve never get cleaned up (for the lifetime of the entity)
// TODO: 0.15 `Task` api is unified, can remove branching
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Used to tie async reactivity tasks to the lifetime of an [`Entity`].
        pub struct TaskHolder(Vec<WasmTaskAdapter>);

        impl Component for TaskHolder {
            const STORAGE_TYPE: StorageType = StorageType::Table;

            fn register_component_hooks(hooks: &mut ComponentHooks) {
                hooks.on_remove(|mut world, entity, _| {
                    for task in world.get_mut::<Self>(entity).unwrap().0.drain(..) {
                        task.0.abort();
                    }
                });
            }
        }
    } else {
        /// Used to tie async reactivity tasks to the lifetime of an [`Entity`].
        #[derive(Component)]
        pub struct TaskHolder(Vec<Task<()>>);
    }
}

impl TaskHolder {
    fn new() -> Self {
        Self(Vec::new())
    }

    // TODO: 0.15 `Task` api is unified, can remove branching
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            /// Drop the [`Task`] when the entity is despawned.
            pub fn hold(&mut self, task: WasmTaskAdapter) {
                self.0.push(task);
            }
        } else {
            /// Drop the [`Task`] when the entity is despawned.
            pub fn hold(&mut self, task: Task<()>) {
                self.0.push(task);
            }
        }
    }
}

fn offset(i: usize, child_block_populations: &[usize]) -> usize {
    child_block_populations[0..i].iter().copied().sum()
}
