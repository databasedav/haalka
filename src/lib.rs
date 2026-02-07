#![doc = include_str!("../README.md")]
//! ## feature flags
#![cfg_attr(
    feature = "document-features",
    doc = document_features::document_features!()
)]

use bevy_app::prelude::*;

use bevy_ecs::schedule::IntoScheduleConfigs;
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
pub mod stripe;
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
///
/// By default, this plugin adds [`JonmoPlugin`](jonmo::JonmoPlugin) with `PostUpdate` as the
/// default schedule. Use [`with_jonmo`](Self::with_jonmo) to add additional schedules:
///
/// ```no_run
/// use bevy::prelude::*;
/// use haalka::prelude::*;
///
/// App::new()
///     .add_plugins(HaalkaPlugin::new().with_jonmo(|jonmo| jonmo.with_schedule::<Update>()));
/// ```
///
/// **Note**: If `JonmoPlugin` is already added, `HaalkaPlugin` will warn and skip adding it.
/// Ensure `JonmoPlugin` is configured with at least `PostUpdate` for proper UI behavior.
pub struct HaalkaPlugin {
    #[allow(clippy::type_complexity)]
    jonmo_configurator:
        bevy_platform::sync::Mutex<Option<Box<dyn FnOnce(jonmo::JonmoPlugin) -> jonmo::JonmoPlugin + Send + Sync>>>,
}

impl Default for HaalkaPlugin {
    fn default() -> Self {
        Self {
            jonmo_configurator: std::sync::Mutex::new(None),
        }
    }
}

impl HaalkaPlugin {
    /// Create a new `HaalkaPlugin` with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the [`JonmoPlugin`](jonmo::JonmoPlugin) with additional schedules.
    ///
    /// The configurator receives a `JonmoPlugin` already set up with `PostUpdate` as
    /// the default schedule. Use this to add additional schedules:
    ///
    /// ```no_run
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// HaalkaPlugin::new().with_jonmo(|jonmo| jonmo.with_schedule::<Update>());
    /// ```
    ///
    /// **Note**: This has no effect if `JonmoPlugin` was already added to the app.
    pub fn with_jonmo(
        self,
        configurator: impl FnOnce(jonmo::JonmoPlugin) -> jonmo::JonmoPlugin + Send + Sync + 'static,
    ) -> Self {
        *self.jonmo_configurator.lock().unwrap() = Some(Box::new(configurator));
        self
    }
}

impl Plugin for HaalkaPlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<jonmo::JonmoPlugin>() {
            bevy_log::warn!(
                "JonmoPlugin was already added before HaalkaPlugin. \
                 Ensure it is configured with at least the PostUpdate schedule for proper UI behavior. \
                 Any .with_jonmo() configuration will not be applied."
            );
        } else {
            let mut jonmo_plugin = jonmo::JonmoPlugin::new::<PostUpdate>();
            if let Some(configurator) = self.jonmo_configurator.lock().unwrap().take() {
                jonmo_plugin = configurator(jonmo_plugin);
            }
            app.add_plugins(jonmo_plugin);
        }
        app.add_plugins((
            align::plugin,
            pointer_event_aware::plugin,
            mouse_wheel_scrollable::plugin,
            viewport_mutable::plugin,
        ));
        app.configure_sets(
            PostUpdate,
            jonmo::SignalProcessing
                .before(bevy_ui::UiSystems::Prepare)
                .before(bevy_text::Text2dUpdateSystems),
        );
        #[cfg(feature = "text_input")]
        app.add_plugins(text_input::plugin);
    }
}

/// `use haalka::prelude::*;` imports everything one needs to use start using [haalka](crate).
pub mod prelude {
    #[doc(inline)]
    pub use crate::HaalkaPlugin;

    // Re-export jonmo crate and prelude
    #[doc(no_inline)]
    pub use crate::jonmo;
    #[doc(no_inline)]
    pub use jonmo::prelude::*;

    #[doc(inline)]
    pub use crate::{
        align::{Align, Alignable},
        column::Column,
        el::El,
        element::{
            AlignabilityFacade, BuilderPassThrough, BuilderWrapper, Element, ElementEither, ElementWrapper,
            IntoElementEither, Nameable, Spawnable, TypeEraseable, UiRoot, UiRootable,
        },
        global_event_aware::{GlobalEventAware, GlobalEventData},
        grid::Grid,
        mouse_wheel_scrollable::{
            BasicScrollHandler, MouseWheelScrollable, OnHoverMouseWheelScrollable, ScrollDirection,
        },
        pointer_event_aware::{
            Cursor, Cursorable, CursorableDisabled, DragData, Draggable, Dragged, Enter, HoverData, Hoverable, Hovered,
            Leave, PointerEventAware, PressData, Pressable, SetCursor, UpdateHoverStatesDisabled,
        },
        row::Row,
        stack::Stack,
        stripe::{self, Stripe},
        viewport_mutable::ViewportMutable,
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
        }
    }
}
