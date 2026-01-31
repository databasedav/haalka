//! High level UI building block/widget abstraction ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Element`](https://github.com/MoonZoon/MoonZoon/blob/f8fc31065f65bdb3ab7b94faf5e3916bc5550dd9/crates/zoon/src/element.rs#L84).

use std::borrow::Cow;

use super::align::Alignable;
use bevy_ecs::{component::*, lifecycle::HookContext, prelude::*, system::RunSystemOnce, world::DeferredWorld};
use bevy_log::warn;
use jonmo::prelude::*;

use bevy_ecs::system::IntoObserverSystem;

/// [`Element`]s are types that wrap [`jonmo::Builder`](jonmo::Builder) and can be aligned using
/// [haalka](crate)'s [simple alignability semantics](super::align::Align) and granted UI-specific
/// abilities like [pointer event awareness](super::pointer_event_aware::PointerEventAware),
/// [viewport mutability](super::viewport_mutable::ViewportMutable),
/// [scrollability](super::mouse_wheel_scrollable::MouseWheelScrollable), etc.
pub trait Element: BuilderWrapper + Alignable {}

impl<E: BuilderWrapper + Alignable> Element for E {}

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

/// The core trait for all UI element types in haalka. Types implementing this trait wrap a
/// [`jonmo::Builder`] and can be used with haalka's UI abilities like
/// [`PointerEventAware`](super::pointer_event_aware::PointerEventAware),
/// [`ViewportMutable`](super::viewport_mutable::ViewportMutable), etc.
///
/// **For primitive elements** (like [`El`](super::el::El), [`Column`](super::column::Column), etc.)
/// that directly hold a [`jonmo::Builder`], implement this trait directly.
///
/// **For widgets** that wrap other elements, implement [`ElementWrapper`] instead, which provides
/// a blanket implementation of `BuilderWrapper` automatically.
pub trait BuilderWrapper: Sized {
    /// Mutable reference to the [`jonmo::Builder`] that this wrapper wraps.
    fn builder_mut(&mut self) -> &mut jonmo::Builder;

    /// Process the wrapped [`jonmo::Builder`] directly.
    fn with_builder(mut self, f: impl FnOnce(jonmo::Builder) -> jonmo::Builder) -> Self {
        let builder = std::mem::take(self.builder_mut());
        *self.builder_mut() = f(builder);
        self
    }

    /// Consume this wrapper, returning the wrapped [`jonmo::Builder`].
    fn into_builder(mut self) -> jonmo::Builder {
        std::mem::take(self.builder_mut())
    }
}

/// Allows [`BuilderWrapper`]s to be spawned into the world.
pub trait Spawnable: BuilderWrapper {
    /// Spawn the element into the world.
    fn spawn(self, world: &mut World) -> Entity {
        self.into_builder().spawn(world)
    }
}

impl<T: BuilderWrapper> Spawnable for T {}

/// [`ElementWrapper`]s can be passed to the child methods of all alignable types, e.g.
/// [`.child`](super::el::El::child), [`.item_signal`](super::column::Column::item_signal),
/// [`.layers`](super::stack::Stack::layers),
/// [`.cells_signal_vec`](super::grid::Grid::cells_signal_vec), etc. This trait provides the
/// foundation for building "widgets" using [haalka](crate).
///
/// # Example
/// ```
/// use bevy::prelude::*;
/// use haalka::prelude::*;
///
/// struct MyWidget {
///     el: El<Node>,
///     data: usize,
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

impl<EW: ElementWrapper> BuilderWrapper for EW {
    fn builder_mut(&mut self) -> &mut jonmo::Builder {
        self.element_mut().builder_mut()
    }

    fn into_builder(self) -> jonmo::Builder {
        self.into_el().into_builder()
    }
}

impl<EW: ElementWrapper> Alignable for EW {}

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
pub trait UiRootable: BuilderWrapper {
    /// Mark this node as the root of the UI tree.
    fn ui_root(self) -> Self {
        self.with_builder(|builder| builder.insert(UiRoot))
    }
}

/// Convenience trait for adding a [`Name`] to an [`Element`].
pub trait Nameable: BuilderWrapper {
    /// Set the [`Name`] of this element.
    fn name<T: Into<Cow<'static, str>>>(mut self, name_option: impl Into<Option<T>>) -> Self {
        if let Some(name) = name_option.into() {
            self = self.with_builder(|builder| builder.insert(Name::new(name)));
        }
        self
    }

    /// Reactively set the name of this element. If the signal outputs [`None`] the [`Name`] is
    /// removed.
    fn name_signal<T, S>(mut self, name_option_signal_option: impl Into<Option<S>>) -> Self
    where
        T: Into<Cow<'static, str>> + Clone + 'static,
        S: Signal<Item = Option<T>> + Send + Sync + 'static,
    {
        if let Some(name_option_signal) = name_option_signal_option.into() {
            self = self.with_builder(|builder| {
                builder.component_signal(name_option_signal.map_in(|name_option| name_option.map(Name::new)))
            });
        }
        self
    }
}

/// Pass-through convenience trait for commonly used [`jonmo::Builder`] methods.
pub trait BuilderPassThrough: BuilderWrapper {
    /// Pass-through for [`jonmo::Builder::lazy_entity`].
    fn lazy_entity(self, entity: jonmo::utils::LazyEntity) -> Self {
        self.with_builder(|builder| builder.lazy_entity(entity))
    }

    /// Pass-through for [`jonmo::Builder::insert`].
    fn insert<T: Bundle>(self, bundle: T) -> Self {
        self.with_builder(|builder| builder.insert(bundle))
    }

    /// Pass-through for [`jonmo::Builder::observe`].
    fn observe<E: EntityEvent, B: Bundle, Marker>(
        self,
        observer: impl IntoObserverSystem<E, B, Marker> + Sync,
    ) -> Self {
        self.with_builder(|builder| builder.observe(observer))
    }

    /// Pass-through for [`jonmo::Builder::component_signal`].
    fn component_signal<C>(self, signal: impl Signal<Item = Option<C>>) -> Self
    where
        C: Component + Clone,
    {
        self.with_builder(|builder| builder.component_signal(signal))
    }
}

/// Generates the `# Clone semantics` doc section for element struct documentation.
///
/// # Usage
/// ```ignore
/// #[doc = clone_semantics_doc!("El")]
/// pub struct El<NodeType> { ... }
/// ```
#[macro_export]
macro_rules! clone_semantics_doc {
    ($type_name:literal) => {
        concat!(
            "# `Clone` semantics\n\n",
            "This type implements [`Clone`] **only** to satisfy trait bounds required by signal combinators.\n",
            "**Cloning `",
            $type_name,
            "`s at runtime is a bug.** See [`",
            $type_name,
            "::clone`] for details."
        )
    };
}

/// Generates the base error text for clone documentation (without trailing punctuation).
#[macro_export]
macro_rules! clone_error_doc_base {
    ($type_name:literal) => {
        concat!(
            "# Error\n\n",
            "This clone implementation exists **only** to satisfy trait bounds required by signal\n",
            "combinators. **Cloning `",
            $type_name,
            "`s at runtime is a bug and will lead to unexpected behavior.**\n\n",
            "Clones produce an empty element with no on-spawn hooks.\n\n",
            "Use factory functions instead if you need reusable UI templates"
        )
    };
}

/// Generates the error docstring for a `Clone::clone` method on element types.
///
/// # Usage
/// ```ignore
/// impl Clone for MyType {
///     #[doc = clone_error_doc!("MyType")]
///     fn clone(&self) -> Self { ... }
/// }
/// ```
///
/// Or with a code example:
/// ```ignore
/// impl Clone for MyType {
///     #[doc = clone_error_doc!("MyType", my_fn, ".method()")]
///     fn clone(&self) -> Self { ... }
/// }
/// ```
#[macro_export]
macro_rules! clone_error_doc {
    // Without example
    ($type_name:literal) => {
        concat!($crate::clone_error_doc_base!($type_name), ".")
    };
    // With example
    ($type_name:literal, $example_fn:ident, $example_method:literal) => {
        concat!(
            $crate::clone_error_doc_base!($type_name),
            ":\n\n",
            "```ignore\n",
            "use bevy_ui::prelude::*;\n",
            "use haalka::prelude::*;\n\n",
            "fn ",
            stringify!($example_fn),
            "(label: &str) -> ",
            $type_name,
            "<Node> {\n",
            "    ",
            $type_name,
            "::new()",
            $example_method,
            "\n",
            "}\n\n",
            "// Correct: each call creates a fresh element\n",
            "let el1 = ",
            stringify!($example_fn),
            "(\"First\");\n",
            "let el2 = ",
            stringify!($example_fn),
            "(\"Second\");\n",
            "```"
        )
    };
}

/// Generates the runtime error message for cloning an element type.
#[macro_export]
macro_rules! clone_error_msg {
    ($type_name:literal) => {
        concat!(
            "Cloning `",
            $type_name,
            "` at {} is a bug! Use a factory function instead."
        )
    };
}

/// Implements [`Clone`] for element types with appropriate errors.
///
/// This macro generates a `Clone` implementation that:
/// - Logs an error at runtime when cloned (since cloning elements is typically a bug)
/// - Has complete documentation explaining the clone semantics
///
/// # Forms
///
/// ## Generic element types with `builder` and `_node_type` fields:
/// ```ignore
/// impl_element_clone! {
///     "El",
///     El<NodeType>,
///     my_el,
///     ".name(\"my_el\")"
/// }
/// ```
///
/// ## Non-generic tuple struct wrapping `jonmo::Builder`:
/// ```ignore
/// impl_element_clone!(simple "AlignabilityFacade", AlignabilityFacade);
/// ```
#[macro_export]
macro_rules! impl_element_clone {
    // Generic element type with builder + _node_type fields and example
    ($type_name:literal, $type:ty, $example_fn:ident, $example_method:literal) => {
        impl<NodeType> Clone for $type {
            #[doc = $crate::clone_error_doc!($type_name, $example_fn, $example_method)]
            #[track_caller]
            fn clone(&self) -> Self {
                let msg = format!(
                    $crate::clone_error_msg!($type_name),
                    std::panic::Location::caller()
                );
                if cfg!(debug_assertions) {
                    let backtrace = std::backtrace::Backtrace::force_capture();
                    panic!("{}\nBacktrace:\n{}", msg, backtrace);
                }
                bevy_log::error!("{}", msg);

                Self {
                    builder: self.builder.clone(),
                    _node_type: std::marker::PhantomData,
                }
            }
        }
    };
    // Simple non-generic tuple struct wrapping jonmo::Builder
    (simple $type_name:literal, $type:ty) => {
        impl Clone for $type {
            #[doc = $crate::clone_error_doc!($type_name)]
            #[track_caller]
            fn clone(&self) -> Self {
                let msg = format!(
                    $crate::clone_error_msg!($type_name),
                    std::panic::Location::caller()
                );
                if cfg!(debug_assertions) {
                    let backtrace = std::backtrace::Backtrace::force_capture();
                    panic!("{}\nBacktrace:\n{}", msg, backtrace);
                }
                bevy_log::error!("{}", msg);

                Self(self.0.clone())
            }
        }
    };
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

impl<T: Alignable> TypeEraseable for T {
    fn type_erase(self) -> AlignabilityFacade {
        AlignabilityFacade(self.into_builder())
    }
}

/// A type-erased [`Element`] that provides a facade of alignability.
///
/// Created via [`TypeEraseable::type_erase`]. The underlying alignment components
/// are preserved from the original element, so alignment behavior works correctly.
#[doc = crate::clone_semantics_doc!("AlignabilityFacade")]
pub struct AlignabilityFacade(jonmo::Builder);

impl_element_clone!(simple "AlignabilityFacade", AlignabilityFacade);

impl BuilderWrapper for AlignabilityFacade {
    fn builder_mut(&mut self) -> &mut jonmo::Builder {
        &mut self.0
    }
}

impl Alignable for AlignabilityFacade {}

/// Enables returning different concrete [`Element`] types from branching logic without type
/// erasure via [`TypeEraseable::type_erase`].
///
/// Inspired by <https://github.com/rayon-rs/either>.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use haalka::prelude::*;
///
/// fn conditional_element(use_column: bool) -> impl Element {
///     if use_column {
///         Column::<Node>::new().left_either()
///     } else {
///         Row::<Node>::new().right_either()
///     }
/// }
/// ```
#[allow(missing_docs)]
pub enum ElementEither<L, R>
where
    L: Element,
    R: Element,
{
    Left(L),
    Right(R),
}

impl<L, R> Clone for ElementEither<L, R>
where
    L: Element + Clone,
    R: Element + Clone,
{
    #[doc = crate::clone_error_doc!("ElementEither")]
    #[track_caller]
    fn clone(&self) -> Self {
        let msg = format!(crate::clone_error_msg!("ElementEither"), std::panic::Location::caller());
        if cfg!(debug_assertions) {
            panic!("{}", msg);
        }
        bevy_log::error!("{}", msg);

        match self {
            Self::Left(left) => Self::Left(left.clone()),
            Self::Right(right) => Self::Right(right.clone()),
        }
    }
}

impl<L, R> BuilderWrapper for ElementEither<L, R>
where
    L: Element,
    R: Element,
{
    fn builder_mut(&mut self) -> &mut jonmo::Builder {
        match self {
            ElementEither::Left(left) => left.builder_mut(),
            ElementEither::Right(right) => right.builder_mut(),
        }
    }

    fn into_builder(self) -> jonmo::Builder {
        match self {
            ElementEither::Left(left) => left.into_builder(),
            ElementEither::Right(right) => right.into_builder(),
        }
    }
}

impl<L, R> Alignable for ElementEither<L, R>
where
    L: Element,
    R: Element,
{
}

/// Blanket trait for transforming [`Element`]s into [`ElementEither::Left`] or
/// [`ElementEither::Right`].
pub trait IntoElementEither: Sized
where
    Self: Element,
{
    /// Wrap this [`Element`] in the [`ElementEither::Left`] variant.
    ///
    /// Useful for conditional branching where different [`Element`] types need to be returned
    /// from the same function or closure.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// fn conditional_child(use_column: bool) -> impl Element {
    ///     if use_column {
    ///         Column::<Node>::new()
    ///             .item(El::<Node>::new())
    ///             .left_either()
    ///     } else {
    ///         Row::<Node>::new()
    ///             .item(El::<Node>::new())
    ///             .right_either()
    ///     }
    /// }
    /// ```
    fn left_either<R>(self) -> ElementEither<Self, R>
    where
        R: Element,
    {
        ElementEither::Left(self)
    }

    /// Wrap this [`Element`] in the [`ElementEither::Right`] variant.
    ///
    /// Useful for conditional branching where different [`Element`] types need to be returned
    /// from the same function or closure.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use haalka::prelude::*;
    ///
    /// fn conditional_child(use_stack: bool) -> impl Element {
    ///     if use_stack {
    ///         Stack::<Node>::new()
    ///             .layer(El::<Node>::new())
    ///             .left_either()
    ///     } else {
    ///         El::<Node>::new()
    ///             .right_either()
    ///     }
    /// }
    /// ```
    fn right_either<L>(self) -> ElementEither<L, Self>
    where
        L: Element,
    {
        ElementEither::Right(self)
    }
}

impl<T: Element> IntoElementEither for T {}
