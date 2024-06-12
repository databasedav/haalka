use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    align::AlignableType, scrollable::Scrollable, AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable,
    IntoOptionElement, PointerEventAware, RawElWrapper, RawHaalkaEl, Sizeable, Stack,
};

pub struct Grid<NodeType> {
    pub(crate) raw_el: RawHaalkaEl,
    pub(crate) align: Option<AlignHolder>,
    pub(crate) _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<NodeType> for Grid<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            raw_el: {
                RawHaalkaEl::from(node_bundle)
                    .with_component::<Style>(|style| {
                        style.display = Display::Grid;
                    })
                    .insert(Pickable::IGNORE)
            },
            align: None,
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> Grid<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Grid<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.raw_el.raw_el_mut()
    }

    fn into_raw_el(self) -> RawHaalkaEl {
        // TODO: why won't grid_template_columns work without a grid wrapper ?
        RawHaalkaEl::from(NodeBundle::default())
            .with_component::<Style>(|style| style.display = Display::Grid)
            .child(self.raw_el)
    }
}

impl<NodeType: Bundle> PointerEventAware for Grid<NodeType> {}
impl<NodeType: Bundle> Scrollable for Grid<NodeType> {}
impl<NodeType: Bundle> Sizeable for Grid<NodeType> {}

// must substract this from the total row width due to float precision shenanigans https://github.com/bevyengine/bevy/issues/12152
pub const GRID_TRACK_FLOAT_PRECISION_SLACK: f32 = 0.0001;

impl<NodeType: Bundle> Grid<NodeType> {
    pub fn row_wrap_cell_width(mut self, cell_width_option: impl Into<Option<f32>>) -> Self {
        if let Some(cell_width) = cell_width_option.into() {
            self.raw_el = self.raw_el.with_component::<Style>(move |style| {
                style.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width);
            });
        }
        self
    }

    pub fn row_wrap_cell_width_signal<S: Signal<Item = impl Into<Option<f32>>> + Send + 'static>(
        mut self,
        cell_width_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cell_width_signal) = cell_width_signal_option.into() {
            self.raw_el = self.raw_el.on_signal_with_component::<Option<f32>, Style>(
                cell_width_signal.map(|cell_width_option| cell_width_option.into()),
                |style, cell_width_option| {
                    if let Some(cell_width) = cell_width_option {
                        style.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width)
                    } else {
                        style.grid_template_columns.clear();
                    }
                },
            );
        }
        self
    }

    pub fn cell<IOE: IntoOptionElement>(mut self, cell_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            cell_option
                .into_option_element()
                .map(|cell| Self::align_child(cell, apply_alignment)),
        );
        self
    }

    pub fn cell_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        cell_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cell_option_signal) = cell_option_signal_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self.raw_el.child_signal(cell_option_signal.map(move |cell_option| {
                cell_option
                    .into_option_element()
                    .map(|cell| Self::align_child(cell, apply_alignment))
            }));
        }
        self
    }

    pub fn cells<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        mut self,
        cells_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(cells_options) = cells_options_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self.raw_el.children(cells_options.into_iter().map(move |cell_option| {
                cell_option
                    .into_option_element()
                    .map(|cell| Self::align_child(cell, apply_alignment))
            }));
        }
        self
    }

    pub fn cells_signal_vec<IOE: IntoOptionElement + 'static, S: SignalVec<Item = IOE> + Send + 'static>(
        mut self,
        cells_options_signal_vec_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cells_options_signal_vec) = cells_options_signal_vec_option.into() {
            let apply_alignment = self.apply_alignment_wrapper();
            self.raw_el = self
                .raw_el
                .children_signal_vec(cells_options_signal_vec.map(move |cell_option| {
                    cell_option
                        .into_option_element()
                        .map(|cell| Self::align_child(cell, apply_alignment))
                }));
        }
        self
    }
}

impl<NodeType: Bundle> Alignable for Grid<NodeType> {
    fn alignable_type(&mut self) -> Option<AlignableType> {
        Some(AlignableType::Grid)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Stack::<NodeType>::apply_content_alignment(style, alignment, action);
    }
}

impl<NodeType: Bundle> ChildAlignable for Grid<NodeType> {
    fn update_style(style: &mut Style) {
        style.display = Display::Grid;
    }

    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Stack::<NodeType>::apply_alignment(style, alignment, action);
    }
}
