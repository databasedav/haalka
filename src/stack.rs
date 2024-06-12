use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    align::AlignableType, scrollable::Scrollable, AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable,
    IntoOptionElement, PointerEventAware, RawElWrapper, RawHaalkaEl, Row, Sizeable,
};

pub struct Stack<NodeType> {
    pub(crate) raw_el: RawHaalkaEl,
    pub(crate) align: Option<AlignHolder>,
    pub(crate) _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<NodeType> for Stack<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            raw_el: {
                RawHaalkaEl::from(node_bundle)
                    .with_component::<Style>(|style| {
                        style.display = Display::Grid;
                        style.grid_auto_columns =
                            GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto);
                        style.grid_auto_rows =
                            GridTrack::minmax(MinTrackSizingFunction::Px(0.), MaxTrackSizingFunction::Auto);
                    })
                    .insert(Pickable::IGNORE)
            },
            align: None,
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> Stack<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Stack<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.raw_el.raw_el_mut()
    }
}

impl<NodeType: Bundle> PointerEventAware for Stack<NodeType> {}
impl<NodeType: Bundle> Scrollable for Stack<NodeType> {}
impl<NodeType: Bundle> Sizeable for Stack<NodeType> {}

impl<NodeType: Bundle> Stack<NodeType> {
    pub fn layer<IOE: IntoOptionElement>(mut self, layer_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            layer_option
                .into_option_element()
                .map(|layer| Self::align_child(layer, apply_alignment)),
        );
        self
    }

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
    fn alignable_type(&mut self) -> Option<AlignableType> {
        Some(AlignableType::Stack)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Row::<NodeType>::apply_content_alignment(style, alignment, action)
    }
}

impl<NodeType: Bundle> ChildAlignable for Stack<NodeType> {
    fn update_style(style: &mut Style) {
        style.grid_column = GridPlacement::start_end(1, 1);
        style.grid_row = GridPlacement::start_end(1, 1);
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
