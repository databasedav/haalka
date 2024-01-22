use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

mod node_builder;
use node_builder::init_async_world;
pub use node_builder::{async_world, NodeBuilder, TaskHolder};

mod raw_el;
pub use raw_el::{
    Element, ElementWrapper, IntoElement, IntoOptionElement, IntoOptionRawElement, IntoRawElement, RawElWrapper,
    RawElement, RawHaalkaEl, Spawnable,
};

mod el;
pub use el::El;

mod column;
pub use column::Column;

mod row;
pub use row::Row;

mod stack;
pub use stack::Stack;

mod grid;
pub use grid::Grid;

mod align;
pub use align::{AddRemove, Align, AlignHolder, Alignable, Alignment, ChildAlignable, ChildProcessable};

mod pointer_event_aware;
pub use pointer_event_aware::PointerEventAware;
use pointer_event_aware::{pressable_system, RiggedPickingPlugin};

mod derive;

mod utils;
pub use utils::{sleep, spawn};

pub use enclose::enclose as clone;
pub use futures_signals_ext::*;
pub use paste::paste;
pub use static_ref_macro::static_ref;

pub use bevy_mod_picking::prelude::*;

pub use apply::{Also, Apply};

pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((AsyncEcsPlugin, RiggedPickingPlugin.build()))
            .add_systems(PreStartup, init_async_world)
            .add_systems(Update, pressable_system);
    }
}
