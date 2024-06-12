use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    align::AlignableType, scrollable::Scrollable, AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable, Column,
    IntoOptionElement, PointerEventAware, RawElWrapper, RawHaalkaEl, Sizeable,
};

pub struct El<NodeType> {
    pub(crate) raw_el: RawHaalkaEl,
    pub(crate) align: Option<AlignHolder>,
    pub(crate) _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<NodeType> for El<NodeType> {
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

impl<NodeType: Bundle + Default> El<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType> RawElWrapper for El<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.raw_el.raw_el_mut()
    }
}

impl<NodeType: Bundle> PointerEventAware for El<NodeType> {}
impl<NodeType: Bundle> Scrollable for El<NodeType> {}
impl<NodeType: Bundle> Sizeable for El<NodeType> {}

impl<NodeType: Bundle> El<NodeType> {
    pub fn child<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            child_option
                .into_option_element()
                .map(|child| Self::align_child(child, apply_alignment)),
        );
        self
    }

    pub fn child_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        child_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(child_option_signal) = child_option_signal_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self.raw_el.child_signal(child_option_signal.map(move |child_option| {
                child_option
                    .into_option_element()
                    .map(|child| Self::align_child(child, apply_alignment))
            }));
        }
        self
    }

    pub fn children<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        mut self,
        child_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(children_options) = child_options_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self
                .raw_el
                .children(children_options.into_iter().map(move |child_option| {
                    child_option
                        .into_option_element()
                        .map(|child| Self::align_child(child, apply_alignment))
                }));
        }
        self
    }

    pub fn children_signal_vec<IOE: IntoOptionElement + 'static, S: SignalVec<Item = IOE> + Send + 'static>(
        mut self,
        children_options_signal_vec_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(children_options_signal_vec) = children_options_signal_vec_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self
                .raw_el
                .children_signal_vec(children_options_signal_vec.map(move |child_option| {
                    child_option
                        .into_option_element()
                        .map(|child| Self::align_child(child, apply_alignment))
                }));
        }
        self
    }
}

impl<NodeType: Bundle> Alignable for El<NodeType> {
    fn alignable_type(&mut self) -> Option<AlignableType> {
        Some(AlignableType::El)
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

impl<NodeType: Bundle> ChildAlignable for El<NodeType> {
    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Column::<NodeType>::apply_alignment(style, alignment, action);
    }
}
