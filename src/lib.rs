use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

mod node_builder;
use node_builder::init_async_world;
pub use node_builder::{async_world, NodeBuilder, TaskHolder};

mod raw_el;
pub use raw_el::{IntoOptionRawElement, IntoRawElement, RawElWrapper, RawElement, RawHaalkaEl, Spawnable};

mod element;
pub use element::{Element, ElementWrapper, IntoElement, IntoOptionElement, NodeTypeIndirector, TypeEraseable};

mod el;
pub use el::El;

mod column;
pub use column::Column;

mod row;
pub use row::Row;

mod stack;
pub use stack::Stack;

mod grid;
pub use grid::{Grid, GRID_TRACK_FLOAT_PRECISION_SLACK};

mod align;
pub use align::{AddRemove, Align, AlignHolder, Alignable, Alignment, ChildAlignable, ChildProcessable};

mod pointer_event_aware;
pub use pointer_event_aware::PointerEventAware;
use pointer_event_aware::{pressable_system, Pressable, RiggedPickingPlugin};

mod derive;

mod utils;
pub use utils::{naive_type_erase, sleep, spawn};

pub use enclose::enclose as clone;
pub use futures_signals_ext::*;
pub use once_cell::sync::Lazy;
pub use paste::paste;

pub use bevy_mod_picking::prelude::*;

pub use apply::{Also, Apply};

pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((AsyncEcsPlugin, RiggedPickingPlugin.build()))
            .add_systems(PreStartup, init_async_world)
            .add_systems(Update, pressable_system.run_if(any_with_component::<Pressable>()));
    }
}
