use super::{
    align::{AlignabilityFacade, Alignable, Aligner, ChildAlignable},
    raw::{RawElWrapper, RawElement, RawHaalkaEl},
};
use bevy::prelude::*;
use bevy_eventlistener::prelude::*;
use bevy_mod_picking::picking_core::Pickable;

/// The high level UI building blocks of [haalka](crate). [`Element`]s are [`RawElement`]s that wrap
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
/// For example one could create a selectable [`Button`](https://github.com/databasedav/haalka/blob/main/examples/challenge01.rs#L66)
/// widget and then [stack them horizontally](https://github.com/databasedav/haalka/blob/main/examples/challenge01.rs#L354)
/// in a [`Row`](super::Row) (or vertically in a [`Column`](super::Column)) and add some
/// [exclusivity logic](https://github.com/databasedav/haalka/blob/main/examples/challenge01.rs#L374)
/// to create a [`RadioGroup`](https://github.com/databasedav/haalka/blob/main/examples/challenge01.rs#L314) widget.
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
/// Since we have an exhaustive list of the possible [`Aligner`]s, we can use a bit type indirection
/// via [`AlignabilityFacade`] to collapse all [`Element`]s and [`ElementWrapper`]s into a single
/// "type erased" type.
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
pub trait UiRootable: Element {
    /// Mark this node as the root of the UI tree.
    fn ui_root(self) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el
                .on_spawn(|world, entity| {
                    world.insert_resource(UiRoot(entity));
                })
                .insert(Pickable::default())
        })
    }
}

impl<E: Element> UiRootable for E {}

// TODO: there should be a way to pass the entity into the system
/// Enables registering "global" event listeners on the [`UiRoot`] node. The [`UiRoot`] must be
/// manually registered with [`UiRootable::ui_root`] for this to work as expected.
pub trait GlobalEventAware: RawElWrapper {
    /// When an `E` [`EntityEvent`] propagates to the [`UiRoot`] node, run a `handler` [`System`].
    fn on_global_event_with_system<E: EntityEvent, Marker>(
        self,
        handler: impl IntoSystem<(), (), Marker> + Send + 'static,
    ) -> Self {
        self.update_raw_el(|raw_el| raw_el.insert_forwarded(ui_root_forwarder, On::<E>::run(handler)))
    }

    /// When an `E` [`EntityEvent`] propagates to the [`UiRoot`] node, run a function with access to
    /// the event's data.
    fn on_global_event<E: EntityEvent>(self, mut handler: impl FnMut(Listener<E>) + Send + Sync + 'static) -> Self {
        self.on_global_event_with_system::<E, _>(move |event: Listener<E>| handler(event))
    }

    /// When an `E` [`EntityEvent`] propagates to the [`UiRoot`] node, run a function with mutable
    /// access to the event's data.
    fn on_global_event_mut<E: EntityEvent>(
        self,
        mut handler: impl FnMut(ListenerMut<E>) + Send + Sync + 'static,
    ) -> Self {
        self.on_global_event_with_system::<E, _>(move |event: ListenerMut<E>| handler(event))
    }
}

fn ui_root_forwarder(entity: &mut EntityWorldMut) -> Option<Entity> {
    entity.world_scope(|world| world.get_resource::<UiRoot>().map(|&UiRoot(ui_root)| ui_root))
}
