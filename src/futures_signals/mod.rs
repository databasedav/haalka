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
pub(crate) use node_builder::init_async_world;
pub use node_builder::{NodeBuilder, TaskHolder, async_world};

pub mod raw;
pub use raw::{
    DeferredUpdaterAppendDirection, HaalkaObserver, HaalkaOneShotSystem, IntoOptionRawElement, IntoRawElement,
    RawElWrapper, RawElement, RawHaalkaEl, Spawnable,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "futures_signals_ui")] {
        pub mod align;
        mod column;
        mod el;
        pub mod element;
        pub mod grid;
        pub mod pointer_event_aware;
        pub mod global_event_aware;
        mod row;
        pub mod mouse_wheel_scrollable;
        mod stack;
        pub mod viewport_mutable;

        cfg_if::cfg_if! {
            if #[cfg(feature = "futures_signals_text_input")] {
                pub mod text_input;
            }
        }
    }
}

#[cfg(feature = "futures_signals_derive")]
pub mod derive;

pub mod utils;

/// Plugin that adds the futures-signals based systems.
pub struct HaalkaFuturesSignalsPlugin;

impl bevy_app::Plugin for HaalkaFuturesSignalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use bevy_app::prelude::*;

        app.add_plugins(bevy_async_ecs::AsyncEcsPlugin);

        #[cfg(feature = "futures_signals_ui")]
        {
            app.add_plugins((
                pointer_event_aware::plugin,
                mouse_wheel_scrollable::plugin,
                viewport_mutable::plugin,
            ));
        }

        #[cfg(feature = "futures_signals_text_input")]
        app.add_plugins(text_input::plugin);

        app.add_systems(PreStartup, init_async_world);
    }
}

/// Prelude for deprecated futures-signals based API.
pub mod prelude {
    #[doc(inline)]
    pub use super::{
        HaalkaFuturesSignalsPlugin,
        node_builder::async_world,
        raw::{RawElWrapper, RawElement, RawHaalkaEl, Spawnable},
    };

    #[doc(no_inline)]
    pub use haalka_futures_signals_ext::*;

    cfg_if::cfg_if! {
        if #[cfg(feature = "futures_signals_ui")] {
            #[doc(inline)]
            pub use super::{
                align::{Align, Alignable},
                column::Column,
                el::El,
                element::{Element, ElementWrapper, Nameable, TypeEraseable, UiRoot, UiRootable},
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
                if #[cfg(feature = "futures_signals_text_input")] {
                    #[doc(inline)]
                    pub use super::text_input::TextInput;
                    pub use bevy_ui_text_input;
                }
            }
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "futures_signals_derive")] {
            #[doc(no_inline)]
            pub use paste::paste;
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "futures_signals_utils")] {
            #[doc(inline)]
            pub use super::utils::*;
            #[doc(no_inline)]
            pub use apply::{Also, Apply};
            pub use std::sync::LazyLock;
        }
    }
}
