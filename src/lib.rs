#![doc = include_str!("../README.md")]

use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

mod node_builder;
use node_builder::init_async_world;

mod raw;
pub use raw::{
    AppendDirection as DeferredUpdateAppendDirection, IntoOptionRawElement, IntoRawElement, RawElWrapper, RawElement,
    RawHaalkaEl, Spawnable,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "ui")] {
        mod align;
        mod column;
        mod el;
        mod element;
        mod grid;
        mod pointer_event_aware;
        mod global_event_aware;
        mod row;
        mod scrollable;
        mod sizeable;
        mod stack;
        mod viewport_mutable;

        pub use self::{
            align::{Align, AlignabilityFacade, Alignable, Alignment, ChildAlignable, Aligner},
            column::Column,
            el::El,
            element::{Element, ElementWrapper, IntoElement, IntoOptionElement, TypeEraseable, UiRoot, UiRootable, Nameable},
            grid::{Grid, GRID_TRACK_FLOAT_PRECISION_SLACK},
            node_builder::{async_world, NodeBuilder, TaskHolder},
            pointer_event_aware::{PointerEventAware, Cursorable},
            global_event_aware::GlobalEventAware,
            row::Row,
            scrollable::{BasicScrollHandler, HoverableScrollable, ScrollDirection, ScrollabilitySettings, Scrollable},
            sizeable::Sizeable,
            stack::Stack,
            viewport_mutable::ViewportMutable,
        };

        use pointer_event_aware::{PointerEventAwarePlugin};
        use scrollable::ScrollablePlugin;

        cfg_if::cfg_if! {
            if #[cfg(feature = "text_input")] {
                /// Reactive text input widget and adjacent utilities, a thin wrapper around [`bevy_cosmic_edit`] integrated with [`Signal`](futures_signals::signal::Signal)s.
                pub mod text_input;
                use text_input::TextInputPlugin;
            }
        }
    }
}

#[cfg(feature = "derive")]
mod derive;

mod utils;

/// Includes the plugins and systems required for [haalka](crate) to function.
///
/// If one is using [`bevy_mod_picking`] directly in their own project or through another, they
/// should add the [`HaalkaPlugin`] *after* any [`bevy_mod_picking`] plugins are added elsewhere as
/// the [`HaalkaPlugin`] checks if its required [`bevy_mod_picking`] plugins are already added
/// before adding them; otherwise, one's app might panic attempting to add duplicate
/// [`bevy_mod_picking`] plugins after the [`HaalkaPlugin`] already has. Additionally, one should
/// ensure the [`bevy_mod_picking`] versions are the same to avoid a similar panic.
pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AsyncEcsPlugin);
        #[cfg(feature = "ui")]
        {
            if !app.is_plugin_added::<bevy_mod_picking::picking_core::CorePlugin>() {
                app.add_plugins(bevy_mod_picking::picking_core::CorePlugin);
            }
            if !app.is_plugin_added::<bevy_mod_picking::picking_core::InteractionPlugin>() {
                app.add_plugins(bevy_mod_picking::picking_core::InteractionPlugin);
            }
            if !app.is_plugin_added::<bevy_mod_picking::input::InputPlugin>() {
                app.add_plugins(bevy_mod_picking::input::InputPlugin);
            }
            if !app.is_plugin_added::<bevy_mod_picking::backends::bevy_ui::BevyUiBackend>() {
                app.add_plugins(bevy_mod_picking::backends::bevy_ui::BevyUiBackend);
            }
            app.add_plugins((PointerEventAwarePlugin, ScrollablePlugin));
        }
        #[cfg(feature = "text_input")]
        app.add_plugins(TextInputPlugin);

        app.add_systems(PreStartup, init_async_world);
    }
}

/// `use haalka::prelude::*;` imports everything one needs to use haalka.
pub mod prelude {
    pub use super::*;

    pub use utils::clone;

    pub use haalka_futures_signals_ext::*;

    pub use bevy_eventlistener::prelude::*;

    cfg_if::cfg_if! {
        if #[cfg(feature = "ui")] {
            pub use paste::paste;
            pub use bevy_mod_picking::{
                events::{
                    Click, Down, Drag, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Drop, Move, Out, Over, Pointer, Up,
                },
                focus::PickingInteraction,
                input::prelude::*,
                picking_core::Pickable,
                pointer::{PointerButton, PointerId, PointerInteraction, PointerLocation, PointerMap, PointerPress},
            };
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "text_input")] {
            pub use super::text_input::{Placeholder, TextAttrs, TextInput};
            pub use bevy_cosmic_edit::{
                CacheKeyFlags, CosmicBackgroundColor, CosmicBackgroundImage, CosmicBuffer, CosmicColor, CosmicPadding,
                CosmicSource, CosmicTextAlign, CosmicTextChanged, CosmicWidgetSize, CosmicWrap, CursorColor, DefaultAttrs,
                FamilyOwned, FocusedWidget as CosmicFocusedWidget, FontStyle, FontWeight, HoverCursor, MaxChars, MaxLines,
                SelectionColor, Stretch, XOffset,
            };
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "utils")] {
            pub use super::utils::{sleep, spawn, sync, sync_neq, flip};
            pub use apply::{Also, Apply};
            pub use once_cell::sync::Lazy;
        }
    }
}
