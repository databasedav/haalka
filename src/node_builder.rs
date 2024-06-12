use std::{convert::identity, sync::OnceLock};

use bevy::{prelude::*, tasks::Task};
use bevy_async_ecs::AsyncWorld;
use enclose::enclose as clone;
use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{MutableVec, SignalVec, SignalVecExt, VecDiff},
};
use futures_signals_ext::{MutableExt, SignalExtExt};
use futures_util::Future;

use crate::spawn;

static ASYNC_WORLD: OnceLock<AsyncWorld> = OnceLock::new();

pub fn async_world() -> &'static AsyncWorld {
    ASYNC_WORLD.get().expect("expected ASYNC_WORLD to be initialized")
}

pub(crate) fn init_async_world(world: &mut World) {
    ASYNC_WORLD
        .set(AsyncWorld::from_world(world))
        .expect("failed to initialize ASYNC_WORLD");
}

#[derive(Default)]
pub struct NodeBuilder {
    on_spawns: Vec<Box<dyn FnOnce(&mut World, Entity) + Send>>,
    task_wrappers: Vec<Box<dyn FnOnce(Entity) -> Task<()> + Send>>,
    contiguous_child_block_populations: MutableVec<Option<usize>>,
}

impl<T: Bundle> From<T> for NodeBuilder {
    fn from(bundle: T) -> Self {
        default::<NodeBuilder>().insert(bundle)
    }
}

impl NodeBuilder {
    pub fn on_spawn(mut self, on_spawn: impl FnOnce(&mut World, Entity) + Send + 'static) -> Self {
        self.on_spawns.push(Box::new(on_spawn));
        self
    }

    pub fn with_entity(self, f: impl FnOnce(EntityWorldMut) + Send + 'static) -> Self {
        self.on_spawn(move |world, entity| {
            if let Some(entity) = world.get_entity_mut(entity) {
                f(entity);
            }
        })
    }

    pub fn insert<B: Bundle>(self, bundle: B) -> Self {
        self.with_entity(|mut entity| {
            entity.insert(bundle);
        })
    }

    pub fn on_signal<T, Fut: Future<Output = ()> + Send + 'static>(
        mut self,
        signal: impl Signal<Item = T> + Send + 'static,
        mut f: impl FnMut(Entity, T) -> Fut + Send + 'static,
    ) -> Self {
        self.task_wrappers.push(Box::new(move |entity: Entity| {
            spawn(signal.for_each(move |value| f(entity, value)))
        }));
        self
    }

    // TODO: list out limitations; limitation: if multiple children are added to entity, they must
    // be registered thru this abstraction because of the way siblings are tracked
    pub fn child(mut self, child: NodeBuilder) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |entity: Entity| {
            spawn(clone!((entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &contiguous_child_block_populations).await;
                }
                async_world().apply(move |world: &mut World| {
                    let child_entity = child.spawn(world);
                    if let Some(mut parent) = world.get_entity_mut(parent) {
                        parent.insert_children(offset.get(), &[child_entity]);
                    } else {  // parent despawned during child spawning
                        if let Some(child) = world.get_entity_mut(child_entity) {
                            child.despawn_recursive();
                        }
                    }
                    contiguous_child_block_populations.lock_mut().set(block, Some(1));
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn child_signal(
        mut self,
        child_option: impl Signal<Item = impl Into<Option<NodeBuilder>> + Send> + Send + 'static,
    ) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let task_wrapper = move |entity: Entity| {
            let offset = offset(block, &contiguous_child_block_populations);
            let existing_child_option = Mutable::new(None);
            spawn(clone!((entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &contiguous_child_block_populations).await;
                }
                child_option.for_each(move |child_option| {
                    clone!((existing_child_option, offset, contiguous_child_block_populations) async move {
                        if let Some(child) = child_option.into() {
                            async_world().apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();  // removes from parent
                                    }
                                }
                                let child_entity = child.spawn(world);
                                if let Some(mut parent) = world.get_entity_mut(parent) {
                                    parent.insert_children(offset.get(), &[child_entity]);
                                    existing_child_option.set(Some(child_entity));
                                } else {  // parent despawned during child spawning
                                    if let Some(child) = world.get_entity_mut(child_entity) {
                                        child.despawn_recursive();
                                    }
                                }
                                contiguous_child_block_populations.lock_mut().set(block, Some(1));
                            }).await;
                        } else {
                            async_world().apply(move |world: &mut World| {
                                if let Some(existing_child) = existing_child_option.take() {
                                    if let Some(entity) = world.get_entity_mut(existing_child) {
                                        entity.despawn_recursive();
                                    }
                                }
                                contiguous_child_block_populations.lock_mut().set(block, Some(0));
                            })
                            .await;
                        }
                    })
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = NodeBuilder> + Send + 'static) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |entity: Entity| {
            spawn(clone!((entity => parent) async move {
                if block > 0 {
                    wait_until_child_block_inserted(block - 1, &contiguous_child_block_populations).await;
                }
                async_world().apply(move |world: &mut World| {
                    let mut children_entities = vec![];
                    for child in children {
                        children_entities.push(child.spawn(world));
                    }
                    let population = children_entities.len();
                    if let Some(mut parent) = world.get_entity_mut(parent) {
                        parent.insert_children(offset.get(), &children_entities);
                    } else {  // parent despawned during child spawning
                        for child in children_entities {
                            if let Some(child) = world.get_entity_mut(child) {
                                child.despawn_recursive();
                            }
                        }
                    }
                    contiguous_child_block_populations.lock_mut().set(block, Some(population));
                }).await;
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn children_signal_vec(
        mut self,
        children_signal_vec: impl SignalVec<Item = NodeBuilder> + Send + 'static,
    ) -> Self {
        let block = self.contiguous_child_block_populations.lock_ref().len();
        self.contiguous_child_block_populations.lock_mut().push(None);
        let contiguous_child_block_populations = self.contiguous_child_block_populations.clone();
        let offset = offset(block, &contiguous_child_block_populations);
        let task_wrapper = move |entity: Entity| {
            spawn(clone!((entity => parent) {
                let children_entities = MutableVec::default();
                children_signal_vec
                .for_each(clone!((parent, children_entities, offset, contiguous_child_block_populations) move |diff| {
                    clone!((parent, children_entities, offset, contiguous_child_block_populations) async move {
                        // TODO: unit tests for every branch
                        match diff {
                            VecDiff::Replace { values: nodes } => {
                                async_world().apply(move |world: &mut World| {
                                    let mut children_lock = children_entities.lock_mut();
                                    let old_children = children_lock.drain(..).collect::<Vec<_>>();
                                    for node in nodes {
                                        children_lock.push(node.spawn(world));
                                    }
                                    for child in old_children {
                                        if let Some(child) = world.get_entity_mut(child) {
                                            child.despawn_recursive();  // removes from parent
                                        }
                                    }
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        parent.insert_children(offset.get(), children_lock.as_slice());
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
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
                            VecDiff::InsertAt { index, value: node } => {
                                async_world().apply(move |world: &mut World| {
                                    let child_entity = node.spawn(world);
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        parent.insert_children(offset.get() + index, &[child_entity]);
                                        let mut children_lock = children_entities.lock_mut();
                                        children_lock.insert(index, child_entity);
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                    } else {  // parent despawned during child spawning
                                        if let Some(child) = world.get_entity_mut(child_entity) {
                                            child.despawn_recursive();
                                        }
                                    }
                                })
                                .await;
                            }
                            VecDiff::Push { value: node } => {
                                async_world().apply(move |world: &mut World| {
                                    let child_entity = node.spawn(world);
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let mut children_lock = children_entities.lock_mut();
                                        parent.insert_children(offset.get() + children_lock.len(), &[child_entity]);
                                        children_lock.push(child_entity);
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
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
                                    let child_entity = node.spawn(world);
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        children_entities.lock_mut().set(index, child_entity);
                                        parent.insert_children(offset.get() + index, &[child_entity]);
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
                                    fn swap(mut parent: &mut EntityWorldMut, children_entities: &[Entity], a: usize, b: usize) {
                                        move_from_to(&mut parent, children_entities, a, b);
                                        if a < b {
                                            move_from_to(&mut parent, children_entities, b - 1, a);

                                        } else if a > b {
                                            move_from_to(&mut parent, children_entities, b + 1, a);
                                        }
                                    }
                                    if let Some(mut parent) = world.get_entity_mut(parent) {
                                        let offset = offset.get();
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
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
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
                                        contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
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
                                    contiguous_child_block_populations.lock_mut().set(block, Some(children_lock.len()));
                                })
                                .await;
                            }
                        }
                    })
                }))
            }))
        };
        self.task_wrappers.push(Box::new(task_wrapper));
        self
    }

    pub fn spawn(self, world: &mut World) -> Entity {
        let id = world.spawn(TaskHolder::new()).id();
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
        id
    }
}

#[derive(Component)]
pub struct TaskHolder(Vec<Task<()>>);

impl TaskHolder {
    fn new() -> Self {
        Self(Vec::new())
    }

    pub fn hold(self: &mut Self, task: Task<()>) {
        self.0.push(task);
    }
}

fn get_offset(i: usize, contiguous_child_block_populations: &[Option<usize>]) -> usize {
    contiguous_child_block_populations[0..i]
        .iter()
        .copied()
        .filter_map(identity)
        .sum()
}

fn offset(i: usize, contiguous_child_block_populations: &MutableVec<Option<usize>>) -> Mutable<usize> {
    let offset = Mutable::new(get_offset(i, &*contiguous_child_block_populations.lock_ref()));
    let updater = {
        contiguous_child_block_populations
            .signal_vec()
            .to_signal_map(move |contiguous_child_block_populations| get_offset(i, contiguous_child_block_populations))
            .dedupe()
            .for_each_sync(clone!((offset) move |new_offset| {
                offset.set_neq(new_offset);
            }))
    };
    spawn(updater).detach(); // future dropped when node is  // TODO: confirm
    offset
}

async fn wait_until_child_block_inserted(block: usize, contiguous_child_block_populations: &MutableVec<Option<usize>>) {
    contiguous_child_block_populations
        .signal_vec()
        .to_signal_map(|contiguous_child_block_populations| {
            contiguous_child_block_populations
                .get(block)
                .copied()
                .flatten()
                .is_some()
        })
        .wait_for(true)
        .await;
}
