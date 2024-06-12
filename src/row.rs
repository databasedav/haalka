use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    align::AlignableType, scrollable::Scrollable, AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable,
    IntoOptionElement, PointerEventAware, RawElWrapper, RawHaalkaEl, Sizeable,
};

pub struct Row<NodeType> {
    pub(crate) raw_el: RawHaalkaEl,
    pub(crate) align: Option<AlignHolder>,
    pub(crate) _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<NodeType> for Row<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            raw_el: {
                RawHaalkaEl::from(node_bundle)
                    .with_component::<Style>(|style| {
                        style.display = Display::Flex;
                        style.flex_direction = FlexDirection::Row;
                        style.align_items = AlignItems::Center;
                    })
                    .insert(Pickable::IGNORE)
            },
            align: None,
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> Row<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Row<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.raw_el.raw_el_mut()
    }
}

impl<NodeType: Bundle> PointerEventAware for Row<NodeType> {}
impl<NodeType: Bundle> Scrollable for Row<NodeType> {}
impl<NodeType: Bundle> Sizeable for Row<NodeType> {}

impl<NodeType: Bundle> Row<NodeType> {
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

    pub fn multiline(mut self) -> Self {
        self.raw_el = self.raw_el.with_component::<Style>(|style| {
            style.flex_wrap = FlexWrap::Wrap;
            style.flex_basis = Val::Px(0.);
            style.flex_grow = 1.;
        });
        self
    }
}

impl<NodeType: Bundle> Alignable for Row<NodeType> {
    fn alignable_type(&mut self) -> Option<AlignableType> {
        Some(AlignableType::Row)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        match alignment {
            Alignment::Top => {
                style.align_items = match action {
                    AddRemove::Add => AlignItems::Start,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::Bottom => {
                style.align_items = match action {
                    AddRemove::Add => AlignItems::End,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
            Alignment::Left => {
                style.justify_content = match action {
                    AddRemove::Add => JustifyContent::Start,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::Right => {
                style.justify_content = match action {
                    AddRemove::Add => JustifyContent::End,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::CenterX => {
                style.justify_content = match action {
                    AddRemove::Add => JustifyContent::Center,
                    AddRemove::Remove => JustifyContent::DEFAULT,
                }
            }
            Alignment::CenterY => {
                style.align_items = match action {
                    AddRemove::Add => AlignItems::Center,
                    AddRemove::Remove => AlignItems::DEFAULT,
                }
            }
        }
    }
}

impl<NodeType: Bundle> ChildAlignable for Row<NodeType> {
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
                style.margin.right = match action {
                    AddRemove::Add => Val::Auto,
                    AddRemove::Remove => Val::ZERO,
                }
            }
            Alignment::Right => {
                style.margin.left = match action {
                    AddRemove::Add => Val::Auto,
                    AddRemove::Remove => Val::ZERO,
                }
            }
            Alignment::CenterX => {
                (style.margin.left, style.margin.right) = match action {
                    AddRemove::Add => (Val::Auto, Val::Auto),
                    AddRemove::Remove => (Val::ZERO, Val::ZERO),
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
