use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

mod node_builder;
use node_builder::init_async_world;
pub use node_builder::{async_world, NodeBuilder, TaskHolder};

mod raw_el;
pub use raw_el::{IntoOptionRawElement, IntoRawElement, RawElWrapper, RawElement, RawHaalkaEl, Spawnable};

mod element;
pub use element::{Element, ElementWrapper, IntoElement, IntoOptionElement, TypeEraseable};

mod el;
pub use el::El;

mod column;
pub use column::Column;

mod row;
pub use row::Row;

mod stack;
use scrollable::ScrollablePlugin;
pub use stack::Stack;

mod grid;
pub use grid::{Grid, GRID_TRACK_FLOAT_PRECISION_SLACK};

mod align;
pub use align::{AddRemove, Align, AlignHolder, AlignabilityFacade, Alignable, Alignment, ChildAlignable};

mod pointer_event_aware;
pub use pointer_event_aware::PointerEventAware;
use pointer_event_aware::{PointerEventAwarePlugin, RiggedPickingPlugin};

mod scrollable;
pub use scrollable::{BasicScrollHandler, HoverableScrollable, ScrollDirection, ScrollabilitySettings, Scrollable};

mod sizable;
pub use sizable::Sizable;

mod derive;

mod utils;
pub use utils::{sleep, spawn};

pub use enclose::enclose as clone;
pub use futures_signals_ext::*;
pub use once_cell::sync::Lazy;
pub use paste::paste;

pub use bevy_mod_picking::prelude::*;

pub use apply::{Also, Apply};

pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AsyncEcsPlugin,
            RiggedPickingPlugin.build(),
            PointerEventAwarePlugin,
            ScrollablePlugin,
        ))
        .add_systems(PreStartup, init_async_world);
    }
}
