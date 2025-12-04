//! Deprecated futures-signals based reactive primitives.
//!
//! This module contains the original futures-signals based implementation of haalka's
//! reactive system. It is deprecated and will be removed in the next major Bevy version.
//!
//! For new code, use jonmo-based reactivity instead.

#![deprecated(
    since = "0.6.0",
    note = "futures-signals backend is deprecated. Use jonmo-based reactivity instead. This module will be removed in the next major Bevy version."
)]

pub mod node_builder;
pub use node_builder::{async_world, NodeBuilder, TaskHolder};
pub(crate) use node_builder::init_async_world;

pub mod raw;
pub use raw::{
    DeferredUpdaterAppendDirection, HaalkaObserver, HaalkaOneShotSystem, IntoOptionRawElement,
    IntoRawElement, RawElWrapper, RawElement, RawHaalkaEl, Spawnable,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "ui")] {
        pub mod align;
        pub mod column;
        pub mod el;
        pub mod element;
        pub mod grid;
        pub mod pointer_event_aware;
        pub mod global_event_aware;
        pub mod row;
        pub mod mouse_wheel_scrollable;
        pub mod stack;
        pub mod viewport_mutable;

        pub use align::{Align, Alignable, AlignX, AlignY, Alignment, ContentAlignment, LayoutDirection};
        pub use column::Column;
        pub use el::El;
        pub use element::{
            AlignabilityFacade, Element, ElementWrapper, IntoElement, IntoOptionElement, Nameable,
            TypeEraseable, UiRoot, UiRootable,
        };
        pub use global_event_aware::GlobalEventAware;
        pub use grid::{Grid, GRID_TRACK_FLOAT_PRECISION_SLACK};
        pub use mouse_wheel_scrollable::{
            BasicScrollHandler, MouseWheelScrollable, OnHoverMouseWheelScrollable, ScrollDirection,
            ScrollDisabled,
        };
        pub use pointer_event_aware::{
            CursorOnHover, CursorOnHoverDisabled, CursorOnHoverable, Enter, Leave, PointerEventAware,
            SetCursor, UpdateHoverStatesDisabled,
        };
        pub use row::Row;
        pub use stack::{Stack, StackChild};
        pub use viewport_mutable::{
            Axis, LogicalRect, MutableViewport, OnViewportLocationChange, Scene, Viewport, ViewportMutable,
        };

        cfg_if::cfg_if! {
            if #[cfg(feature = "text_input")] {
                pub mod text_input;
                pub use text_input::{ClearSelectionOnFocusChangeDisabled, TextInput};
            }
        }
    }
}

#[cfg(feature = "derive")]
pub mod derive;

pub mod utils;

/// Plugin that adds the futures-signals based systems.
pub struct FuturesSignalsPlugin;

impl bevy_app::Plugin for FuturesSignalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use bevy_app::prelude::*;
        
        app.add_plugins(bevy_async_ecs::AsyncEcsPlugin);
        
        #[cfg(feature = "ui")]
        {
            app.add_plugins((
                align::plugin,
                pointer_event_aware::plugin,
                mouse_wheel_scrollable::plugin,
                viewport_mutable::plugin,
            ));
        }
        
        #[cfg(feature = "text_input")]
        app.add_plugins(text_input::plugin);

        app.add_systems(PreStartup, init_async_world);
    }
}

/// Prelude for deprecated futures-signals based API.
pub mod prelude {
    #[doc(inline)]
    pub use super::{
        FuturesSignalsPlugin,
        node_builder::async_world,
        raw::{RawElWrapper, RawElement, RawHaalkaEl, Spawnable},
    };

    #[doc(no_inline)]
    pub use haalka_futures_signals_ext::*;

    cfg_if::cfg_if! {
        if #[cfg(feature = "ui")] {
            #[doc(inline)]
            pub use super::{
                align::{Align, Alignable},
                column::Column,
                el::El,
                element::{AlignabilityFacade, Element, ElementWrapper, Nameable, TypeEraseable, UiRoot, UiRootable},
                global_event_aware::GlobalEventAware,
                grid::Grid,
                mouse_wheel_scrollable::{
                    BasicScrollHandler, MouseWheelScrollable, OnHoverMouseWheelScrollable, ScrollDirection,
                },
                pointer_event_aware::{SetCursor, CursorOnHoverDisabled, CursorOnHoverable, PointerEventAware, Enter, Leave},
                row::Row,
                stack::Stack,
                viewport_mutable::{Axis, ViewportMutable},
            };

            pub use bevy_window::{SystemCursorIcon, CursorIcon};

            cfg_if::cfg_if! {
                if #[cfg(feature = "text_input")] {
                    #[doc(inline)]
                    pub use super::text_input::TextInput;
                    pub use bevy_ui_text_input;
                }
            }
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "derive")] {
            #[doc(no_inline)]
            pub use paste::paste;
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "utils")] {
            #[doc(inline)]
            pub use super::utils::*;
            #[doc(no_inline)]
            pub use apply::{Also, Apply};
            pub use std::sync::LazyLock;
        }
    }
}
