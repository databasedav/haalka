use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    align::AlignableType, scrollable::Scrollable, AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable,
    IntoOptionElement, PointerEventAware, RawElWrapper, RawHaalkaEl, Sizable,
};

pub struct Column<NodeType> {
    pub(crate) raw_el: RawHaalkaEl,
    pub(crate) align: Option<AlignHolder>,
    pub(crate) _node_type: std::marker::PhantomData<NodeType>,
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
                    .insert(Pickable::IGNORE)
            },
            align: None,
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> Column<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Column<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.raw_el.raw_el_mut()
    }
}

impl<NodeType: Bundle> PointerEventAware for Column<NodeType> {}
impl<NodeType: Bundle> Scrollable for Column<NodeType> {}
impl<NodeType: Bundle> Sizable for Column<NodeType> {}

impl<NodeType: Bundle> Column<NodeType> {
    pub fn item<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            child_option
                .into_option_element()
                .map(|child| Self::align_child(child, apply_alignment)),
        );
        self
    }

    pub fn item_signal<IOE: IntoOptionElement + 'static>(
        mut self,
        child_option: impl Signal<Item = IOE> + Send + 'static,
    ) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child_signal(child_option.map(move |child_option| {
            child_option
                .into_option_element()
                .map(|child| Self::align_child(child, apply_alignment))
        }));
        self
    }

    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self
            .raw_el
            .children(children_options.into_iter().map(move |child_option| {
                child_option
                    .into_option_element()
                    .map(|child| Self::align_child(child, apply_alignment))
            }));
        self
    }

    pub fn items_signal_vec<IOE: IntoOptionElement + 'static>(
        mut self,
        children_options_signal_vec: impl SignalVec<Item = IOE> + Send + 'static,
    ) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self
            .raw_el
            .children_signal_vec(children_options_signal_vec.map(move |child_option| {
                child_option
                    .into_option_element()
                    .map(|child| Self::align_child(child, apply_alignment))
            }));
        self
    }
}

impl<NodeType: Bundle> Alignable for Column<NodeType> {
    fn alignable_type(&self) -> Option<AlignableType> {
        Some(AlignableType::Column)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => {
                style.justify_content = match action {
                    AddRemove::Add => JustifyContent::Start,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::Bottom => {
                style.justify_content = match action {
                    AddRemove::Add => JustifyContent::End,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::Left => {
                style.align_items = match action {
                    AddRemove::Add => AlignItems::Start,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::Right => {
                style.align_items = match action {
                    AddRemove::Add => AlignItems::End,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::CenterX => {
                style.align_items = match action {
                    AddRemove::Add => AlignItems::Center,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::CenterY => {
                style.justify_content = match action {
                    AddRemove::Add => JustifyContent::Center,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
        }
    }
}

impl<NodeType: Bundle> ChildAlignable for Column<NodeType> {
    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => {
                style.margin.bottom = match action {
                    AddRemove::Add => Val::Auto,
                    AddRemove::Remove => Val::ZERO,
                }
            }
            Alignment::Bottom => {
                style.margin.top = match action {
                    AddRemove::Add => Val::Auto,
                    AddRemove::Remove => Val::ZERO,
                }
            }
            Alignment::Left => {
                style.align_self = match action {
                    AddRemove::Add => AlignSelf::Start,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::Right => {
                style.align_self = match action {
                    AddRemove::Add => AlignSelf::End,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::CenterX => {
                style.align_self = match action {
                    AddRemove::Add => AlignSelf::Center,
                    AddRemove::Remove => AlignSelf::DEFAULT,
                }
            }
            Alignment::CenterY => {
                (style.margin.top, style.margin.bottom) = match action {
                    AddRemove::Add => (Val::Auto, Val::Auto),
                    AddRemove::Remove => (Val::ZERO, Val::ZERO),
                }
            }
        }
    }
}
