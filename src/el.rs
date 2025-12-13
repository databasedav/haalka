use bevy_ecs::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use jonmo::{
    builder::JonmoBuilder,
    signal::{Signal, SignalExt},
};

use super::{
    align::{Alignable, LayoutDirection},
    element::{BuilderWrapper, IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{Hoverable, Pressable, CursorOnHoverable, PointerEventAware},
    viewport_mutable::ViewportMutable,
};

// TODO: add the extra flag machinery that MoonZoon has to ensure that El's have exactly one child
// (or child signal)
/// Singleton [`Element`](super::element::Element) with exactly one child (not yet enforced). Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`El`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/el.rs).
///
/// While multiple children can still be declared with repeated calls to [`.child`](`El::child`) or
/// [`.child_signal`](`El::child_signal`), their relative alignment was arbitrarily chosen to match
/// [MoonZoon's implementation](https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/el.rs#L41-L69) and should not be relied on.
#[derive(Default)]
pub struct El<NodeType> {
    builder: JonmoBuilder,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<JonmoBuilder> for El<NodeType> {
    fn from(builder: JonmoBuilder) -> Self {
        Self {
            builder: builder
                .with_component::<Node>(|mut node| {
                    node.display = Display::Flex;
                    node.flex_direction = FlexDirection::Column;
                })
                .insert((LayoutDirection::Column, Pickable::IGNORE, Hoverable, Pressable)),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> El<NodeType> {
    /// Construct a new [`El`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(JonmoBuilder::from(NodeType::default()))
    }

    /// Construct a new [`El`] from a [`Bundle`].
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn from_bundle(node_bundle: NodeType) -> Self {
        Self::from(JonmoBuilder::from(node_bundle))
    }
}

impl<NodeType> BuilderWrapper for El<NodeType> {
    fn builder_mut(&mut self) -> &mut JonmoBuilder {
        &mut self.builder
    }
}

impl<NodeType: Bundle> CursorOnHoverable for El<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for El<NodeType> {}
impl<NodeType: Bundle> Nameable for El<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for El<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for El<NodeType> {}
impl<NodeType: Bundle> UiRootable for El<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for El<NodeType> {}

impl<NodeType: Bundle> El<NodeType> {
    /// Declare a static child.
    pub fn child<IOE: IntoOptionElement>(self, child_option: IOE) -> Self {
        if let Some(child) = child_option.into_option_element() {
            self.with_builder(|builder| builder.child(child.into_builder()))
        } else {
            self
        }
    }

    /// Declare a reactive child. When the [`Signal`] outputs [`None`], the child is removed.
    pub fn child_signal<IOE, S>(self, child_option_signal_option: impl Into<Option<S>>) -> Self
    where
        IOE: IntoOptionElement + 'static,
        S: Signal<Item = IOE> + Send + Sync + 'static,
    {
        if let Some(child_option_signal) = child_option_signal_option.into() {
            self.with_builder(|builder| {
                builder.child_signal(
                    child_option_signal.map_in(move |child_option: IOE| {
                        child_option.into_option_element().map(|el| el.into_builder())
                    }),
                )
            })
        } else {
            self
        }
    }
}

impl<NodeType: Bundle> Alignable for El<NodeType> {
    fn layout_direction() -> LayoutDirection {
        LayoutDirection::Column
    }
}
