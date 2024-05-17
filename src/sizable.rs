use crate::{raw_el::AppendDirection, RawElWrapper};
use bevy::prelude::*;
use futures_signals::signal::Signal;

pub trait Sizable: RawElWrapper {
    fn height(self, height: Val) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                raw_el.with_component::<Style>(move |style| style.height = height)
            })
        })
    }

    fn height_signal(self, height_signal: impl Signal<Item = Val> + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                raw_el.on_signal_with_component::<Val, Style>(height_signal, move |style, height| style.height = height)
            })
        })
    }

    fn width(self, width: Val) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                raw_el.with_component::<Style>(move |style| style.width = width)
            })
        })
    }

    fn width_signal(self, width_signal: impl Signal<Item = Val> + Send + 'static) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.defer_update(AppendDirection::Back, move |raw_el| {
                raw_el.on_signal_with_component::<Val, Style>(width_signal, move |style, width| style.width = width)
            })
        })
    }
}
