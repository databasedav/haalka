use super::raw::{DeferredUpdaterAppendDirection, RawElWrapper};
use bevy::prelude::*;
use futures_signals::signal::{Signal, SignalExt};

/// Enables an element to have a static or reactive vertical and horizontal length, with
/// consideration for any potential [haalka](crate) managed wrapper nodes.
///
/// For example, [`Grid`](super::grid::Grid)s, [`Stack`](super::stack::Stack)s, and [mutable
/// viewport](super::viewport_mutable::ViewportMutable)s use wrapper elements to manage the expected
/// state of their body. Modifying the [`Style::height`] or [`Style::width`] of such elements
/// directly may not have the desired effect.
pub trait Sizeable: RawElWrapper {
    /// Set the height of this element.
    fn height(mut self, height_option: impl Into<Option<Val>>) -> Self {
        if let Some(height) = height_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
                    raw_el.with_component::<Style>(move |mut style| style.height = height)
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
                    raw_el.on_signal_with_component::<Option<Val>, Style>(
                        height_option_signal,
                        move |mut style, height_option| style.height = height_option.unwrap_or(Val::Auto),
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
                    raw_el.with_component::<Style>(move |mut style| style.width = width)
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
                    raw_el.on_signal_with_component::<Option<Val>, Style>(
                        width_option_signal,
                        move |mut style, width_option| style.width = width_option.unwrap_or(Val::Auto),
                    )
                })
            });
        }
        self
    }
}
