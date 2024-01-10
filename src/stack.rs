use bevy::prelude::*;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable, ChildProcessable,
    IntoOptionElement, RawElWrapper, RawElement, RawHaalkaEl, Row,
};

pub struct Stack<NodeType> {
    raw_el: RawHaalkaEl<NodeType>,
    align: Option<AlignHolder>,
}

impl<NodeType: Bundle> From<NodeType> for Stack<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            raw_el: {
                RawHaalkaEl::from(node_bundle).with_component::<Style>(|style| {
                    style.display = Display::Grid;
                    style.grid_auto_columns = vec![GridTrack::minmax(
                        MinTrackSizingFunction::Px(0.),
                        MaxTrackSizingFunction::Auto,
                    )];
                    style.grid_auto_rows = vec![GridTrack::minmax(
                        MinTrackSizingFunction::Px(0.),
                        MaxTrackSizingFunction::Auto,
                    )];
                })
            },
            align: None,
        }
    }
}

impl<NodeType: Bundle + Default> Stack<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> Stack<NodeType> {
    pub fn layer<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where
        <IOE::EL as RawElement>::NodeType: Bundle,
        IOE::EL: ChildProcessable,
    {
        self.raw_el = self.raw_el.child(Self::process_child(child_option));
        self
    }

    pub fn layer_signal<IOE: IntoOptionElement + 'static>(
        mut self,
        child_option_signal: impl Signal<Item = IOE> + Send + 'static,
    ) -> Self
    where
        <IOE::EL as RawElement>::NodeType: Bundle,
        IOE::EL: ChildProcessable,
    {
        self.raw_el = self
            .raw_el
            .child_signal(child_option_signal.map(Self::process_child));
        self
    }

    pub fn layers<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        mut self,
        children_options: I,
    ) -> Self
    where
        <IOE::EL as RawElement>::NodeType: Bundle,
        I::IntoIter: Send + 'static,
        IOE::EL: ChildProcessable,
    {
        self.raw_el = self
            .raw_el
            .children(children_options.into_iter().map(Self::process_child));
        self
    }

    pub fn layers_signal_vec<IOE: IntoOptionElement + 'static>(
        mut self,
        children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static,
    ) -> Self
    where
        <IOE::EL as RawElement>::NodeType: Bundle,
        IOE::EL: ChildProcessable,
    {
        self.raw_el = self
            .raw_el
            .children_signal_vec(children_options_signal_vec.map(Self::process_child));
        self
    }
}

impl<NodeType: Bundle> RawElWrapper for Stack<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.raw_el.raw_el_mut()
    }
}

impl<NodeType: Bundle> Alignable for Stack<NodeType> {
    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Row::<NodeType>::apply_content_alignment(style, alignment, action)
    }
}

impl<NodeType: Bundle> ChildAlignable for Stack<NodeType> {
    fn update_style(style: &mut Style) {
        style.grid_column = GridPlacement::start(1);
        style.grid_row = GridPlacement::start(1);
    }

    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => {
                style.align_self = match action {
                    AddRemove::Add => AlignSelf::Start,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::Bottom => {
                style.align_self = match action {
                    AddRemove::Add => AlignSelf::End,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::Left => {
                style.justify_self = match action {
                    AddRemove::Add => JustifySelf::Start,
                    AddRemove::Remove => JustifySelf::DEFAULT,
                }
            }
            Alignment::Right => {
                style.justify_self = match action {
                    AddRemove::Add => JustifySelf::End,
                    AddRemove::Remove => JustifySelf::DEFAULT,
                }
            }
            Alignment::CenterX => {
                style.justify_self = match action {
                    AddRemove::Add => JustifySelf::Center,
                    AddRemove::Remove => JustifySelf::DEFAULT,
                }
            }
            Alignment::CenterY => {
                style.align_self = match action {
                    AddRemove::Add => AlignSelf::Center,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
        }
    }
}
