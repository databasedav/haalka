#![doc = include_str!("../README.md")]

use bevy::prelude::*;
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
        // #[allow(missing_docs)]
        pub mod pointer_event_aware;
        // #[allow(missing_docs)]
        pub mod global_event_aware;
        mod row;
        // #[allow(missing_docs)]
        pub mod mouse_wheel_scrollable;
        // #[allow(missing_docs)]
        pub mod sizeable;
        mod stack;
        // #[allow(missing_docs)]
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
        node_builder::async_world,
        raw::{RawElWrapper, RawElement, RawHaalkaEl, Spawnable},
        HaalkaPlugin,
    };

    #[doc(no_inline)]
    pub use haalka_futures_signals_ext::*;

    #[doc(no_inline)]
    pub use bevy_eventlistener::prelude::*;

    cfg_if::cfg_if! {
        if #[cfg(feature = "ui")] {
            #[doc(no_inline)]
            pub use paste::paste;
            pub use bevy_mod_picking;

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
                pointer_event_aware::{SetCursor, CursorOnHoverDisabled, CursorOnHoverable, PointerEventAware},
                row::Row,
                sizeable::Sizeable,
                stack::Stack,
                viewport_mutable::{LimitToBody, ViewportMutable},
            };

            cfg_if::cfg_if! {
                if #[cfg(feature = "text_input")] {
                    #[doc(inline)]
                    pub use super::text_input::{Placeholder, TextAttrs, TextInput};
                    pub use bevy_cosmic_edit;
                }
            }
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "utils")] {
            #[doc(inline)]
            pub use crate::utils::*;
            #[doc(no_inline)]
            pub use apply::{Also, Apply};
            #[doc(no_inline)]
            pub use once_cell::sync::Lazy;
        }
    }
}
