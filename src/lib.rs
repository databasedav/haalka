#![doc = include_str!("../README.md")]
//! ## feature flags
#![cfg_attr(
    feature = "document-features",
    doc = document_features::document_features!()
)]

use bevy_app::prelude::*;

use bevy_ecs::schedule::IntoScheduleConfigs;
// Re-export jonmo for direct access
pub use jonmo;

pub mod align;
mod column;
mod el;
pub mod element;
pub mod global_event_aware;
pub mod grid;
pub mod mouse_wheel_scrollable;
pub mod pointer_event_aware;
mod row;
mod stack;
pub mod viewport_mutable;

#[cfg(feature = "text_input")]
pub mod text_input;

#[cfg(feature = "derive")]
mod derive;

#[allow(missing_docs)]
pub mod utils;

/// Deprecated futures-signals based reactive primitives.
/// Use jonmo-based reactivity instead for new code.
#[cfg(feature = "futures_signals")]
#[allow(deprecated)]
pub mod futures_signals;

/// Includes the plugins and systems required for [haalka](crate) to function.
pub struct HaalkaPlugin;

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(jonmo::JonmoPlugin::new().in_schedule(PostUpdate));
        app.add_plugins((
            align::plugin,
            pointer_event_aware::plugin,
            mouse_wheel_scrollable::plugin,
            viewport_mutable::plugin,
        ));
        app.configure_sets(
            PostUpdate,
            (jonmo::SignalProcessing, bevy_ui::UiSystems::Prepare).chain(),
        );
        #[cfg(feature = "text_input")]
        app.add_plugins(text_input::plugin);
    }
}

/// `use haalka::prelude::*;` imports everything one needs to use start using [haalka](crate).
pub mod prelude {
    #[doc(inline)]
    pub use crate::HaalkaPlugin;

    // Re-export jonmo prelude
    #[doc(no_inline)]
    pub use jonmo::prelude::*;

    // Re-export JonmoBuilder as the main builder type
    #[doc(inline)]
    pub use jonmo::builder::JonmoBuilder;

    #[doc(inline)]
    pub use crate::{
        align::{Align, Alignable},
        column::Column,
        el::El,
        element::{
            AlignabilityFacade, BuilderWrapper, Element, ElementWrapper, Nameable, Spawnable, TypeEraseable, UiRoot,
            UiRootable,
        },
        global_event_aware::GlobalEventAware,
        grid::Grid,
        mouse_wheel_scrollable::{
            BasicScrollHandler, MouseWheelScrollable, OnHoverMouseWheelScrollable, ScrollDirection,
        },
        pointer_event_aware::{
            CursorOnHoverDisabled, CursorOnHoverable, Enter, Hovered, Leave, PointerEventAware, SetCursor,
        },
        row::Row,
        stack::Stack,
        viewport_mutable::{Axis, ViewportMutable},
    };

    pub use bevy_window::{CursorIcon, SystemCursorIcon};

    #[cfg(feature = "text_input")]
    #[doc(inline)]
    pub use super::text_input::TextInput;
    #[cfg(feature = "text_input")]
    pub use bevy_ui_text_input;

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
