use bevy_ecs::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use super::{
    align::{Alignable, LayoutDirection},
    element::{IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, PointerEventAware},
    raw::{RawElWrapper, RawHaalkaEl},
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::element::Element) with vertically stacked children. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Column`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/column.rs).
#[derive(Default)]
pub struct Column<NodeType> {
    raw_el: RawHaalkaEl,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<RawHaalkaEl> for Column<NodeType> {
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

impl<NodeType: Bundle> From<NodeType> for Column<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        RawHaalkaEl::from(node_bundle).into()
    }
}

impl<NodeType: Bundle + Default> Column<NodeType> {
    /// Construct a new [`Column`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Column<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.raw_el
    }
}

impl<NodeType: Bundle> CursorOnHoverable for Column<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Column<NodeType> {}
impl<NodeType: Bundle> Nameable for Column<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for Column<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for Column<NodeType> {}
impl<NodeType: Bundle> UiRootable for Column<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Column<NodeType> {}

impl<NodeType: Bundle> Column<NodeType> {
    /// Declare a static vertically stacked child.
    pub fn item<IOE: IntoOptionElement>(mut self, item_option: IOE) -> Self {
        self.raw_el = self.raw_el.child(item_option.into_option_element());
        self
    }

    /// Declare a reactive vertically stacked child. When the [`Signal`] outputs [`None`], the child
    /// is removed.
    pub fn item_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        item_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(item_option_signal) = item_option_signal_option.into() {
            self.raw_el = self
                .raw_el
                .child_signal(item_option_signal.map(move |item_option| item_option.into_option_element()));
        }
        self
    }

    /// Declare static vertically stacked children.
    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        mut self,
        items_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(items_options) = items_options_option.into() {
            self.raw_el = self.raw_el.children(
                items_options
                    .into_iter()
                    .map(move |item_option| item_option.into_option_element()),
            );
        }
        self
    }

    /// Declare reactive vertically stacked children.
    pub fn items_signal_vec<IOE: IntoOptionElement + 'static, S: SignalVec<Item = IOE> + Send + 'static>(
        mut self,
        items_options_signal_vec_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(items_options_signal_vec) = items_options_signal_vec_option.into() {
            self.raw_el = self.raw_el.children_signal_vec(
                items_options_signal_vec.map(move |item_option| item_option.into_option_element()),
            );
        }
        self
    }
}

impl<NodeType: Bundle> Alignable for Column<NodeType> {
    fn layout_direction() -> LayoutDirection {
        LayoutDirection::Column
    }
}
