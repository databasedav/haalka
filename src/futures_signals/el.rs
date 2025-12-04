use bevy_ecs::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use futures_signals::signal::{Signal, SignalExt};

use super::{
    align::{Alignable, LayoutDirection},
    element::{IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, PointerEventAware},
    raw::{RawElWrapper, RawHaalkaEl},
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
    raw_el: RawHaalkaEl,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<RawHaalkaEl> for El<NodeType> {
    fn from(value: RawHaalkaEl) -> Self {
        Self {
            raw_el: value
                .with_component::<Node>(|mut node| {
                    node.display = Display::Flex;
                    node.flex_direction = FlexDirection::Column;
                })
                .insert(Pickable::IGNORE)
                .insert(LayoutDirection::Column),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle> From<NodeType> for El<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        RawHaalkaEl::from(node_bundle).into()
    }
}

impl<NodeType: Bundle + Default> El<NodeType> {
    /// Construct a new [`El`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType> RawElWrapper for El<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.raw_el
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
    pub fn child<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self {
        self.raw_el = self.raw_el.child(child_option.into_option_element());
        self
    }

    /// Declare a reactive child. When the [`Signal`] outputs [`None`], the child is removed.
    pub fn child_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        child_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(child_option_signal) = child_option_signal_option.into() {
            self.raw_el = self
                .raw_el
                .child_signal(child_option_signal.map(move |child_option| child_option.into_option_element()));
        }
        self
    }
}

impl<NodeType: Bundle> Alignable for El<NodeType> {
    fn layout_direction() -> LayoutDirection {
        LayoutDirection::Column
    }
}
