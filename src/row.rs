use bevy_ecs::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use super::{
    align::{AddRemove, AlignHolder, Alignable, Aligner, Alignment, ChildAlignable},
    element::{IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, PointerEventAware},
    raw::{RawElWrapper, RawHaalkaEl},
    sizeable::Sizeable,
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::element::Element) with horizontally stacked children. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Row`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/row.rs).
#[derive(Default)]
pub struct Row<NodeType> {
    raw_el: RawHaalkaEl,
    align: Option<AlignHolder>,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<RawHaalkaEl> for Row<NodeType> {
    fn from(value: RawHaalkaEl) -> Self {
        Self {
            raw_el: value
                .with_component::<Node>(|mut node| {
                    node.display = Display::Flex;
                    node.flex_direction = FlexDirection::Row;
                    node.align_items = AlignItems::Center;
                })
                .insert(PickingBehavior::IGNORE),
            align: None,
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle> From<NodeType> for Row<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        RawHaalkaEl::from(node_bundle).into()
    }
}

impl<NodeType: Bundle + Default> Row<NodeType> {
    /// Construct a new [`Row`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Row<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.raw_el
    }
}

impl<NodeType: Bundle> CursorOnHoverable for Row<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Row<NodeType> {}
impl<NodeType: Bundle> Nameable for Row<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for Row<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for Row<NodeType> {}
impl<NodeType: Bundle> Sizeable for Row<NodeType> {}
impl<NodeType: Bundle> UiRootable for Row<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Row<NodeType> {}

impl<NodeType: Bundle> Row<NodeType> {
    /// Declare a static horizontally stacked child.
    pub fn item<IOE: IntoOptionElement>(mut self, item_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            item_option
                .into_option_element()
                .map(|item| Self::align_child(item, apply_alignment)),
        );
        self
    }

    /// Declare a reactive horizontally stacked child. When the [`Signal`] outputs [`None`], the
    /// child is removed.
    pub fn item_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        item_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(item_option_signal) = item_option_signal_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self.raw_el.child_signal(item_option_signal.map(move |item_option| {
                item_option
                    .into_option_element()
                    .map(|item| Self::align_child(item, apply_alignment))
            }));
        }
        self
    }

    /// Declare static horizontally stacked children.
    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        mut self,
        items_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(items_options) = items_options_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self.raw_el.children(items_options.into_iter().map(move |item_option| {
                item_option
                    .into_option_element()
                    .map(|item| Self::align_child(item, apply_alignment))
            }));
        }
        self
    }

    /// Declare reactive horizontally stacked children.
    pub fn items_signal_vec<IOE: IntoOptionElement + 'static, S: SignalVec<Item = IOE> + Send + 'static>(
        mut self,
        items_options_signal_vec_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(items_options_signal_vec) = items_options_signal_vec_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self
                .raw_el
                .children_signal_vec(items_options_signal_vec.map(move |item_option| {
                    item_option
                        .into_option_element()
                        .map(|item| Self::align_child(item, apply_alignment))
                }));
        }
        self
    }

    /// When the width of the row exceeds the width of its parent, wrap the row's children to the
    /// next line, recursively.
    pub fn multiline(mut self) -> Self {
        self.raw_el = self.raw_el.with_component::<Node>(|mut node| {
            node.flex_wrap = FlexWrap::Wrap;
            node.flex_basis = Val::Px(0.);
            node.flex_grow = 1.;
        });
        self
    }
}

impl<NodeType: Bundle> Alignable for Row<NodeType> {
    fn aligner(&mut self) -> Option<Aligner> {
        Some(Aligner::Row)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(node: &mut Node, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => {
                node.align_items = match action {
                    AddRemove::Add => AlignItems::Start,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::Bottom => {
                node.align_items = match action {
                    AddRemove::Add => AlignItems::End,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::Left => {
                node.justify_content = match action {
                    AddRemove::Add => JustifyContent::Start,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::Right => {
                node.justify_content = match action {
                    AddRemove::Add => JustifyContent::End,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::CenterX => {
                node.justify_content = match action {
                    AddRemove::Add => JustifyContent::Center,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::CenterY => {
                node.align_items = match action {
                    AddRemove::Add => AlignItems::Center,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
        }
    }
}

impl<NodeType: Bundle> ChildAlignable for Row<NodeType> {
    fn apply_alignment(node: &mut Node, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => {
                node.align_self = match action {
                    AddRemove::Add => AlignSelf::Start,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::Bottom => {
                node.align_self = match action {
                    AddRemove::Add => AlignSelf::End,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::Left => {
                node.margin.right = match action {
                    AddRemove::Add => Val::Auto,
                    AddRemove::Remove => Val::ZERO,
                }
            }
            Alignment::Right => {
                node.margin.left = match action {
                    AddRemove::Add => Val::Auto,
                    AddRemove::Remove => Val::ZERO,
                }
            }
            Alignment::CenterX => {
                (node.margin.left, node.margin.right) = match action {
                    AddRemove::Add => (Val::Auto, Val::Auto),
                    AddRemove::Remove => (Val::ZERO, Val::ZERO),
                }
            }
            Alignment::CenterY => {
                node.align_self = match action {
                    AddRemove::Add => AlignSelf::Center,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
        }
    }
}
