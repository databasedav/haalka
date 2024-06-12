use crate::{raw_el::AppendDirection, RawElWrapper};
use bevy::prelude::*;
use futures_signals::signal::{Signal, SignalExt};

pub trait Sizeable: RawElWrapper {
    fn height(mut self, height_option: impl Into<Option<Val>>) -> Self {
        if let Some(height) = height_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                    raw_el.with_component::<Style>(move |style| style.height = height)
                })
            });
        }
        self
    }

    fn height_signal<S: Signal<Item = impl Into<Option<Val>>> + Send + 'static>(
        mut self,
        height_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(height_option_signal) = height_option_signal_option.into() {
            let height_option_signal = height_option_signal.map(|height_option| height_option.into());
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                    raw_el.on_signal_with_component::<Option<Val>, Style>(
                        height_option_signal,
                        move |style, height_option| style.height = height_option.unwrap_or(Val::Auto),
                    )
                })
            });
        }
        self
    }

    fn width(mut self, width_option: impl Into<Option<Val>>) -> Self {
        if let Some(width) = width_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                    raw_el.with_component::<Style>(move |style| style.width = width)
                })
            });
        }
        self
    }

    fn width_signal<S: Signal<Item = Val> + Send + 'static>(
        mut self,
        width_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(width_option_signal) = width_option_signal_option.into() {
            let width_option_signal = width_option_signal.map(|width_option| width_option.into());
            self = self.update_raw_el(|raw_el| {
                raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                    raw_el.on_signal_with_component::<Option<Val>, Style>(
                        width_option_signal,
                        move |style, width_option| style.width = width_option.unwrap_or(Val::Auto),
                    )
                })
            });
        }
        self
    }
}
