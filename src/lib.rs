#![doc = include_str!("../README.md")]
//! ## feature flags
#![cfg_attr(
    feature = "document-features",
    doc = document_features::document_features!()
)]

use bevy_app::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

pub mod node_builder;
use node_builder::init_async_world;

pub mod raw;

cfg_if::cfg_if! {
    if #[cfg(feature = "ui")] {
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
            if #[cfg(feature = "text_input")] {
                pub mod text_input;
            }
        }
    }
}

#[cfg(feature = "derive")]
mod derive;

#[allow(missing_docs)]
pub mod utils;

/// Includes the plugins and systems required for [haalka](crate) to function.
pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AsyncEcsPlugin);
        #[cfg(feature = "ui")]
        {
            app.add_plugins((
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

/// `use haalka::prelude::*;` imports everything one needs to use start using [haalka](crate).
pub mod prelude {
    #[doc(inline)]
    pub use crate::{
        HaalkaPlugin,
        node_builder::async_world,
        raw::{RawElWrapper, RawElement, RawHaalkaEl, Spawnable},
    };

    #[doc(no_inline)]
    pub use haalka_futures_signals_ext::*;

    cfg_if::cfg_if! {
        if #[cfg(feature = "ui")] {
            #[doc(inline)]
            pub use crate::{
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
            pub use crate::utils::*;
            #[doc(no_inline)]
            pub use apply::{Also, Apply};
            pub use std::sync::LazyLock;
        }
    }
}
