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
    row::Row,
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::element::Element) with children stacked on directly on top of each other (e.g. along the z-axis), with siblings ordered youngest to oldest, top to bottom. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Stack`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/stack.rs).
#[derive(Default)]
pub struct Stack<NodeType> {
    raw_el: RawHaalkaEl,
    align: Option<AlignHolder>,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<RawHaalkaEl> for Stack<NodeType> {
    fn from(value: RawHaalkaEl) -> Self {
        Self {
            raw_el: value
                .with_component::<Node>(|mut node| {
                    node.display = Display::Grid;
                    node.grid_auto_columns =
                        GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto);
                    node.grid_auto_rows =
                        GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto);
                })
                .insert(Pickable::IGNORE),
            align: None,
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle> From<NodeType> for Stack<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        RawHaalkaEl::from(node_bundle).into()
    }
}

impl<NodeType: Bundle + Default> Stack<NodeType> {
    /// Construct a new [`Stack`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Stack<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.raw_el
    }
}

impl<NodeType: Bundle> CursorOnHoverable for Stack<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Stack<NodeType> {}
impl<NodeType: Bundle> Nameable for Stack<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for Stack<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for Stack<NodeType> {}
impl<NodeType: Bundle> UiRootable for Stack<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Stack<NodeType> {}

impl<NodeType: Bundle> Stack<NodeType> {
    /// Declare a static z-axis stacked child, e.g. subsequent calls to [`.layer`][Stack::layer]s
    /// will be stacked on top of this one.
    pub fn layer<IOE: IntoOptionElement>(mut self, layer_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            layer_option
                .into_option_element()
                .map(|layer| Self::align_child(layer, apply_alignment)),
        );
        self
    }

    /// Declare a reactive z-axis stacked child. When the [`Signal`] outputs [`None`], the child is
    /// removed.
    pub fn layer_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        layer_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(layer_option_signal) = layer_option_signal_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self.raw_el.child_signal(layer_option_signal.map(move |layer_option| {
                layer_option
                    .into_option_element()
                    .map(|layer| Self::align_child(layer, apply_alignment))
            }));
        }
        self
    }

    /// Declare static z-axis stacked children.
    pub fn layers<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        mut self,
        layers_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(layers_options) = layers_options_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self
                .raw_el
                .children(layers_options.into_iter().map(move |layer_option| {
                    layer_option
                        .into_option_element()
                        .map(|layer| Self::align_child(layer, apply_alignment))
                }));
        }
        self
    }

    /// Declare reactive z-axis stacked children.
    pub fn layers_signal_vec<IOE: IntoOptionElement + 'static, S: SignalVec<Item = IOE> + Send + 'static>(
        mut self,
        layers_options_signal_vec_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(layers_options_signal_vec) = layers_options_signal_vec_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self
                .raw_el
                .children_signal_vec(layers_options_signal_vec.map(move |layer_option| {
                    layer_option
                        .into_option_element()
                        .map(|layer| Self::align_child(layer, apply_alignment))
                }));
        }
        self
    }
}

impl<NodeType: Bundle> Alignable for Stack<NodeType> {
    fn aligner(&mut self) -> Option<Aligner> {
        Some(Aligner::Stack)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(node: &mut Node, alignment: Alignment, action: AddRemove) {
        Row::<NodeType>::apply_content_alignment(node, alignment, action)
    }
}

impl<NodeType: Bundle> ChildAlignable for Stack<NodeType> {
    fn update_node(mut node: Mut<Node>) {
        node.grid_column = GridPlacement::start_end(1, 1);
        node.grid_row = GridPlacement::start_end(1, 1);
    }

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
                node.justify_self = match action {
                    AddRemove::Add => JustifySelf::Start,
                    AddRemove::Remove => JustifySelf::DEFAULT,
                }
            }
            Alignment::Right => {
                node.justify_self = match action {
                    AddRemove::Add => JustifySelf::End,
                    AddRemove::Remove => JustifySelf::DEFAULT,
                }
            }
            Alignment::CenterX => {
                node.justify_self = match action {
                    AddRemove::Add => JustifySelf::Center,
                    AddRemove::Remove => JustifySelf::DEFAULT,
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
