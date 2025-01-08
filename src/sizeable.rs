//! Semantics for managing elements' static or reactive vertical and horizontal length, integrated
//! with the wrapper elements that [haalka](crate) employs, see [`Sizeable`].

use super::raw::{DeferredUpdaterAppendDirection, RawElWrapper};
use bevy_ui::prelude::*;
use futures_signals::signal::{Signal, SignalExt};

/// Enables an element to have a static or reactive vertical and horizontal length, with
/// consideration for any potential [haalka](crate) managed wrapper nodes.
///
/// For example, [`Grid`](super::grid::Grid)s, [`Stack`](super::stack::Stack)s, and [mutable
/// viewport](super::viewport_mutable::ViewportMutable)s use wrapper elements to manage the expected
/// state of their body. Modifying the [`Node::height`] or [`Node::width`] of such elements
/// directly may not have the desired effect.
pub trait Sizeable: RawElWrapper {
    /// Set the height of this element.
    fn height(mut self, height_option: impl Into<Option<Val>>) -> Self {
        if let Some(height) = height_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
                    raw_el.with_component::<Node>(move |mut node| node.height = height)
                })
            });
        }
        self
    }

    /// Reactively set the height of this element. If the signal outputs [`None`] the height is set
    /// to [`Val::Auto`].
    fn height_signal<S: Signal<Item = impl Into<Option<Val>>> + Send + 'static>(
        mut self,
        height_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(height_option_signal) = height_option_signal_option.into() {
            let height_option_signal = height_option_signal.map(|height_option| height_option.into());
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
                    raw_el.on_signal_with_component::<Option<Val>, Node>(
                        height_option_signal,
                        move |mut node, height_option| node.height = height_option.unwrap_or(Val::Auto),
                    )
                })
            });
        }
        self
    }

    /// Set the width of this element.
    fn width(mut self, width_option: impl Into<Option<Val>>) -> Self {
        if let Some(width) = width_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
                    raw_el.with_component::<Node>(move |mut node| node.width = width)
                })
            });
        }
        self
    }

    /// Reactively set the width of this element. If the signal outputs [`None`] the width is set to
    /// [`Val::Auto`].
    fn width_signal<S: Signal<Item = Val> + Send + 'static>(
        mut self,
        width_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(width_option_signal) = width_option_signal_option.into() {
            let width_option_signal = width_option_signal.map(|width_option| width_option.into());
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
                    raw_el.on_signal_with_component::<Option<Val>, Node>(
                        width_option_signal,
                        move |mut node, width_option| node.width = width_option.unwrap_or(Val::Auto),
                    )
                })
            });
        }
        self
    }
}
