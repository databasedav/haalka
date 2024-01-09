use bevy::prelude::*;
use futures_signals::{signal::{Signal, SignalExt}, signal_vec::{SignalVec, SignalVecExt}};

use crate::{RawHaalkaEl, AlignHolder, RawElWrapper, IntoOptionElement, RawElement, ChildAlignable, ChildProcessable, Alignment, AddRemove, Alignable};

pub struct Column<NodeType> {
    raw_el: RawHaalkaEl<NodeType>,
    align: Option<AlignHolder>,
}

impl<NodeType: Bundle> From<NodeType> for Column<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            raw_el: {
                RawHaalkaEl::from(node_bundle)
                .with_component::<Style>(|style| {
                    style.display = Display::Flex;
                    style.flex_direction = FlexDirection::Column;
                })
            },
            align: None,
        }
    }
}

impl<NodeType: Bundle + Default> Column<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> Column<NodeType> {
    pub fn item<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where <IOE::EL as RawElement>::NodeType: Bundle, IOE::EL: ChildProcessable
    {
        self.raw_el = self.raw_el.child(Self::process_child(child_option));
        self
    }

    pub fn item_signal<IOE: IntoOptionElement + 'static>(mut self, child_option: impl Signal<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as RawElement>::NodeType: Bundle, IOE::EL: ChildProcessable
    {
        self.raw_el = self.raw_el.child_signal(child_option.map(Self::process_child));
        self
    }

    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
    where <IOE::EL as RawElement>::NodeType: Bundle, I::IntoIter: Send + 'static, IOE::EL: ChildProcessable
    {
        self.raw_el = self.raw_el.children(children_options.into_iter().map(Self::process_child));
        self
    }

    pub fn items_signal_vec<IOE: IntoOptionElement + 'static>(mut self, children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static) -> Self
    where <IOE::EL as RawElement>::NodeType: Bundle, IOE::EL: ChildProcessable
    {
        self.raw_el = self.raw_el.children_signal_vec(children_options_signal_vec.map(Self::process_child));
        self
    }
}

impl<NodeType: Bundle> Alignable for Column<NodeType> {
    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }
    
    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Start,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Bottom => style.justify_content = match action {
                AddRemove::Add => JustifyContent::End,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
            Alignment::Left => style.align_items = match action {
                AddRemove::Add => AlignItems::Start,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::Right => style.align_items = match action {
                AddRemove::Add => AlignItems::End,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::CenterX => style.align_items = match action {
                AddRemove::Add => AlignItems::Center,
                AddRemove::Remove => AlignItems::DEFAULT,
            },
            Alignment::CenterY => style.justify_content = match action {
                AddRemove::Add => JustifyContent::Center,
                AddRemove::Remove => JustifyContent::DEFAULT,
            },
        }
    }
}

impl<NodeType: Bundle> ChildAlignable for Column<NodeType> {
    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => style.margin.bottom = match action {
                AddRemove::Add => Val::Auto,
                AddRemove::Remove => Val::ZERO,
            },
            Alignment::Bottom => style.margin.top = match action {
                AddRemove::Add => Val::Auto,
                AddRemove::Remove => Val::ZERO,
            },
            Alignment::Left => style.align_self = match action {
                AddRemove::Add => AlignSelf::Start,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::Right => style.align_self = match action {
                AddRemove::Add => AlignSelf::End,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::CenterX => style.align_self = match action {
                AddRemove::Add => AlignSelf::Center,
                AddRemove::Remove => AlignSelf::DEFAULT,
            },
            Alignment::CenterY => (style.margin.top, style.margin.bottom) = match action {
                AddRemove::Add => (Val::Auto, Val::Auto),
                AddRemove::Remove => (Val::ZERO, Val::ZERO),
            },
        }
    }
}

impl<NodeType: Bundle> RawElWrapper for Column<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.raw_el.raw_el_mut()
    }
}
