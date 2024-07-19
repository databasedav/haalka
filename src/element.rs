use std::borrow::Cow;

use super::{
    align::{AlignabilityFacade, Alignable, Aligner, ChildAlignable},
    raw::{RawElWrapper, RawElement, RawHaalkaEl},
};
use bevy::prelude::*;
use futures_signals::signal::{Signal, SignalExt};

/// The high level UI building block of [haalka](crate). [`Element`]s are [`RawElement`]s that wrap
/// [bevy_ui nodes](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ui/src/node_bundles.rs)
/// and be can be aligned using [haalka](crate)'s [simple alignability semantics](super::Align) and
/// granted UI-specific abilities like [pointer event awareness](super::PointerEventAware),
/// [scrollability](super::Scrollable), [viewport mutability](super::ViewportMutable), etc.
pub trait Element: RawElement + Alignable + ChildAlignable {}

impl<E: RawElement + Alignable + ChildAlignable> Element for E {}

/// Allows consumers to pass non-[`ElementWrapper`] types to the child methods of all alignable
/// types.
pub trait IntoElement {
    /// The type of the [`Element`] that this type is converted into.
    type EL: Element;
    /// Convert this type into an [`Element`].
    fn into_element(self) -> Self::EL;
}

impl<T: Element> IntoElement for T {
    type EL = T;
    fn into_element(self) -> Self::EL {
        self
    }
}

// impl IntoElement for &'static str {
//     type EL = El<TextBundle>;
//     fn into_element(self) -> Self::EL {
//         El::<TextBundle>::new().text(Text::from_section(
//             self.to_string(),
//             TextStyle::default(),
//         ))
//     }
// }

/// Thin wrapper trait around [`Element`] that allows consumers to pass [`Option`]s to the child
/// methods of all alignable types.
pub trait IntoOptionElement {
    /// The type of the [`Element`] that this type is maybe converted into.
    type EL: Element;
    /// Maybe convert this type into an [`Element`].
    fn into_option_element(self) -> Option<Self::EL>;
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for Option<IE> {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        self.map(|into_element| into_element.into_element())
    }
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for IE {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        Some(self.into_element())
    }
}

/// [`ElementWrapper`]s can be passed to the child methods of all alignable types, e.g.
/// [`.child`](super::El::child), [`.item_signal`](super::Column::item_signal),
/// [`.layers`](super::Stack::layers), [`.cells_signal_vec`](super::Grid::cells_signal_vec), etc.
/// This trait provides the foundation for building "widgets" using [haalka](crate).
///
/// For example one could create a selectable [`Button`](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L83)
/// widget and then [stack them horizontally](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L354)
/// in a [`Row`](super::Row) (or vertically in a [`Column`](super::Column)) and add some
/// [exclusivity logic](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L374)
/// to create a [`RadioGroup`](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L314) widget.
///
/// [`ElementWrapper`]s can also be granted UI-specific abilities, enabling consumers to easily add
/// additional functionality to their custom widgets.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use haalka::prelude::*;
///
/// struct MyWidget {
///     el: El<NodeBundle>,
///     data: Mutable<usize>,
/// }
///
/// impl ElementWrapper for MyWidget {
///     type EL = El<NodeBundle>;
///     fn element_mut(&mut self) -> &mut Self::EL {
///         &mut self.el
///     }
/// }
///
/// impl PointerEventAware for MyWidget {}
/// impl Scrollable for MyWidget {}
/// impl Sizeable for MyWidget {}
/// impl ViewportMutable for MyWidget {}
/// ```
pub trait ElementWrapper: Sized {
    /// The type of the [`Element`] that this wrapper wraps; this can be another [`ElementWrapper`].
    type EL: Element;
    /// Mutable reference to the [`Element`] that this wrapper wraps.
    fn element_mut(&mut self) -> &mut Self::EL;
}

impl<EW: ElementWrapper> RawElWrapper for EW {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.element_mut().raw_el_mut()
    }
}

/// Enables mixing of different types of [`Element`]s.
///
/// Since [`Element`]s or [`ElementWrapper::EL`]s can be of different concrete types (e.g.
/// `El<NodeBundle>`, `El<ImageBundle>`, `Column<NodeBundle>`, etc.), one will run into unfortunate
/// type issues when doing things like returning differnt [`ElementWrapper`]s (read: widgets) from
/// diverging branches of logic, or creating a collection of [`ElementWrapper`]s of different types.
/// Since we have an exhaustive list of the possible [`Aligner`]s, we can use a bit of type
/// indirection via [`AlignabilityFacade`] to collapse all [`Element`]s and [`ElementWrapper`]s into
/// a single "type erased" type.
pub trait TypeEraseable {
    /// Convert this type into an [`AlignabilityFacade`], allowing it to mix with other types of
    /// [`Element`]s and [`ElementWrapper`]s.
    fn type_erase(self) -> AlignabilityFacade;
}

impl<T: Alignable> TypeEraseable for T {
    fn type_erase(mut self) -> AlignabilityFacade {
        let aligner = self.aligner().unwrap_or(Aligner::El);
        let (align_option, raw_el) = (self.align_mut().take(), self.into_raw());
        AlignabilityFacade::new(raw_el, align_option, aligner)
    }
}

/// A resource for holding the root [`Entity`] of the UI tree.
///
/// Used to register global event listeners.
#[derive(Resource)]
pub struct UiRoot(pub Entity);

/// Allows [`Element`]s to be marked as the root of the UI tree.
pub trait UiRootable: RawElWrapper {
    /// Mark this node as the root of the UI tree.
    fn ui_root(self) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.on_spawn(|world, entity| {
                world.insert_resource(UiRoot(entity));
            })
        })
    }
}

/// Convenience trait for adding a [`Name`] to an [`Element`].
pub trait Nameable: RawElWrapper {
    /// Set the [`Name`] of this element.
    fn name<T: Into<Cow<'static, str>>>(mut self, name_option: impl Into<Option<T>>) -> Self {
        if let Some(name) = name_option.into() {
            self = self.update_raw_el(|raw_el| raw_el.insert(Name::new(name)));
        }
        self
    }

    /// Reactively set the name of this element. If the signal outputs [`None`] the [`Name`] is
    /// removed.
    fn name_signal<T: Into<Cow<'static, str>> + 'static, S: Signal<Item = Option<T>> + Send + 'static>(
        mut self,
        name_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(name_option_signal) = name_option_signal_option.into() {
            self = self.update_raw_el(|raw_el| {
                raw_el.component_signal::<Name, _>(name_option_signal.map(|name_option| name_option.map(Name::new)))
            });
        }
        self
    }
}
