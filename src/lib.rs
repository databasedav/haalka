use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

mod node_builder;
use node_builder::init_async_world;
pub use node_builder::{async_world, NodeBuilder, TaskHolder};

mod raw_el;
pub use raw_el::{IntoOptionRawElement, IntoRawElement, RawElWrapper, RawElement, RawHaalkaEl, Spawnable};

mod element;
pub use element::{Element, ElementWrapper, IntoElement, IntoOptionElement, TypeEraseable, UiRoot, UiRootable};

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

mod sizeable;
pub use sizeable::Sizeable;

mod viewport_mutable;
pub use viewport_mutable::ViewportMutable;

mod text_input;
use text_input::TextInputPlugin;
pub use text_input::{PlaceHolder, TextAttrs, TextInput};

mod derive;

mod utils;
pub use utils::{sleep, spawn};

pub use apply::{Also, Apply};
pub use bevy_cosmic_edit::{
    CacheKeyFlags, CosmicBackgroundColor, CosmicBackgroundImage, CosmicBuffer, CosmicColor, CosmicPadding,
    CosmicSource, CosmicTextAlign, CosmicTextChanged, CosmicWidgetSize, CosmicWrap, CursorColor, DefaultAttrs,
    FamilyOwned, FocusedWidget as CosmicFocusedWidget, FontStyle, FontWeight, HoverCursor, MaxChars, MaxLines,
    SelectionColor, Stretch, XOffset,
};
pub use bevy_mod_picking::prelude::*;
pub use enclose::enclose as clone;
pub use futures_signals_ext::*;
pub use once_cell::sync::Lazy;
pub use paste::paste;

pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AsyncEcsPlugin,
            RiggedPickingPlugin.build(),
            PointerEventAwarePlugin,
            ScrollablePlugin,
            TextInputPlugin,
        ))
        .add_systems(PreStartup, init_async_world);
    }
}
