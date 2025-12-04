//! High level UI building block/widget abstraction ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Element`](https://github.com/MoonZoon/MoonZoon/blob/f8fc31065f65bdb3ab7b94faf5e3916bc5550dd9/crates/zoon/src/element.rs#L84).

use std::borrow::Cow;

use super::{
    align::Alignable,
    raw::{RawElWrapper, RawElement, RawHaalkaEl},
};
use bevy_ecs::{component::*, lifecycle::HookContext, prelude::*, system::RunSystemOnce, world::DeferredWorld};
use bevy_log::warn;
use bevy_picking::prelude::*;
use futures_signals::signal::{Signal, SignalExt};

/// [`Element`]s are [`RawElement`]s that wrap [bevy_ui nodes](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ui/src/node_bundles.rs)
/// and be can be aligned using [haalka](crate)'s [simple alignability
/// semantics](super::align::Align) and granted UI-specific abilities like [pointer event
/// awareness](super::pointer_event_aware::PointerEventAware), [viewport
/// mutability](super::viewport_mutable::ViewportMutable),
/// [scrollability](super::mouse_wheel_scrollable::MouseWheelScrollable), etc.
pub trait Element: RawElement + Alignable {}

impl<E: RawElement + Alignable> Element for E {}

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
//     type EL = El<Text>;
//     fn into_element(self) -> Self::EL {
//         El::<Text>::new().text(Text::from_section(
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
/// [`.child`](super::el::El::child), [`.item_signal`](super::column::Column::item_signal),
/// [`.layers`](super::stack::Stack::layers),
/// [`.cells_signal_vec`](super::grid::Grid::cells_signal_vec), etc. This trait provides the
/// foundation for building "widgets" using [haalka](crate).
///
/// For example one could create a selectable [`Button`](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L83)
/// widget and then [stack them horizontally](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L354)
/// in a [`Row`](super::row::Row) (or vertically in a [`Column`](super::column::Column)) and add
/// some [exclusivity logic](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L374)
/// to create a [`RadioGroup`](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/challenge01.rs#L314) widget.
///
/// [`ElementWrapper`]s can also be granted UI-specific abilities, enabling consumers to easily add
/// additional functionality to their custom widgets.
///
/// # Example
/// ```
/// use bevy::prelude::*;
/// use haalka::prelude::*;
///
/// struct MyWidget {
///     el: El<Node>,
///     data: Mutable<usize>,
/// }
///
/// impl ElementWrapper for MyWidget {
///     type EL = El<Node>;
///     fn element_mut(&mut self) -> &mut Self::EL {
///         &mut self.el
///     }
/// }
///
/// impl GlobalEventAware for MyWidget {}
/// impl PointerEventAware for MyWidget {}
/// impl ViewportMutable for MyWidget {}
/// impl MouseWheelScrollable for MyWidget {}
/// ```
pub trait ElementWrapper: Sized {
    /// The type of the [`Element`] that this wrapper wraps; this can be another [`ElementWrapper`].
    type EL: Element + Default;
    /// Mutable reference to the [`Element`] that this wrapper wraps.
    fn element_mut(&mut self) -> &mut Self::EL;

    /// Indirection which allows trait consumers to define custom "build" or "render" logic outside
    /// the body of the [`ElementWrapper`] itself, allowing the [`ElementWrapper`] to be more
    /// ergonomically used as a configuration builder.
    ///
    /// Couldn't figure out how to do this without the [`Default`] constraint since it is required
    /// by [`mem::take`], and was led to the [`mem::take`] solution since there didn't seem to
    /// be a viable unsafe way to take ownership of a single field of a struct with only a
    /// mutable reference to the field, i.e. via [`.element_mut()`](ElementWrapper::element_mut).
    ///
    /// [`mem::take`]: std::mem::take
    fn into_el(mut self) -> Self::EL {
        std::mem::take(self.element_mut())
    }
}

impl<EW: ElementWrapper> RawElWrapper for EW {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.element_mut().raw_el_mut()
    }

    fn into_raw_el(self) -> RawHaalkaEl {
        self.into_el().into_raw()
    }
}

impl<EW: ElementWrapper> Alignable for EW {
    fn layout_direction() -> super::align::LayoutDirection {
        EW::EL::layout_direction()
    }
}

/// Enables mixing of different types of [`Element`]s.
///
/// Since [`Element`]s or [`ElementWrapper::EL`]s can be of different concrete types (e.g.
/// `El<Node>`, `El<ImageBundle>`, `Column<Node>`, etc.), one will run into unfortunate
/// type issues when doing things like returning different [`ElementWrapper`]s (read: widgets) from
/// diverging branches of logic, or creating a collection of [`ElementWrapper`]s of different types.
/// This trait allows collapsing all [`Element`]s and [`ElementWrapper`]s into a single
/// "type erased" [`AlignabilityFacade`] type that still implements [`Element`].
pub trait TypeEraseable {
    /// Convert this type into an [`AlignabilityFacade`], allowing it to mix with other types of
    /// [`Element`]s and [`ElementWrapper`]s.
    fn type_erase(self) -> AlignabilityFacade;
}

impl<T: RawElWrapper> TypeEraseable for T {
    fn type_erase(self) -> AlignabilityFacade {
        AlignabilityFacade(self.into_raw_el())
    }
}

/// A type-erased [`Element`] that provides a facade of alignability.
///
/// Created via [`TypeEraseable::type_erase`]. The underlying [`LayoutDirection`] component
/// is preserved from the original element, so alignment behavior works correctly.
/// Alignment methods should be called *before* type erasure, not after.
///
/// [`LayoutDirection`]: super::align::LayoutDirection
pub struct AlignabilityFacade(RawHaalkaEl);

impl RawElWrapper for AlignabilityFacade {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.0
    }
}

impl Alignable for AlignabilityFacade {
    fn layout_direction() -> super::align::LayoutDirection {
        panic!(
            "AlignabilityFacade::layout_direction() should never be called. \
             Alignment methods should be called before type erasure, not after."
        )
    }
}

fn warn_non_orphan_ui_root(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world.commands().queue(move |world: &mut World| {
        let _ = world.run_system_once(move |child_ofs: Query<&ChildOf>| {
            if child_ofs.iter_ancestors(entity).count() > 0 {
                warn!(
                    "entity {:?} is registered as a UiRoot but is not an orphan (has a parent); this may lead to unexpected behavior",
                    entity
                );
            }
        });
    })
}

/// Marker component for the root of the UI tree. Use [`UiRootable::ui_root`] to
/// register an [`Element`] as the [`UiRoot`].
///
/// Used to register global event listeners.
#[derive(Component)]
#[component(on_add = warn_non_orphan_ui_root)]
pub struct UiRoot;

/// Allows [`Element`]s to be marked as the root of the UI tree.
pub trait UiRootable: RawElWrapper {
    /// Mark this node as the root of the UI tree.
    fn ui_root(self) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert(UiRoot).insert(Pickable::default()))
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
