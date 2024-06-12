use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    align::AlignableType, scrollable::Scrollable, AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable,
    IntoOptionElement, PointerEventAware, RawElWrapper, RawHaalkaEl, Sizeable, ViewportMutable,
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
impl<NodeType: Bundle> Sizeable for Column<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Column<NodeType> {}

impl<NodeType: Bundle> Column<NodeType> {
    pub fn item<IOE: IntoOptionElement>(mut self, item_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            item_option
                .into_option_element()
                .map(|item| Self::align_child(item, apply_alignment)),
        );
        self
    }

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
}

impl<NodeType: Bundle> Alignable for Column<NodeType> {
    fn alignable_type(&mut self) -> Option<AlignableType> {
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
