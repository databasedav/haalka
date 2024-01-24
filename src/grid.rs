use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use crate::{
    AddRemove, AlignHolder, Alignable, Alignment, ChildAlignable, ChildProcessable, IntoOptionElement,
    PointerEventAware, RawElWrapper, RawElement, RawHaalkaEl, Stack,
};

pub struct Grid<NodeType> {
    raw_el: RawHaalkaEl<NodeType>,
    align: Option<AlignHolder>,
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
        }
    }
}

impl<NodeType: Bundle + Default> Grid<NodeType> {
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle + Default> RawElWrapper for Grid<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self.raw_el.raw_el_mut()
    }

    fn into_raw_el(self) -> RawHaalkaEl<NodeType> {
        // TODO: why won't grid_template_columns work without a grid wrapper ?
        // this forces me to require `NodeType: Default` so i can create the appropriate wrapper node
        // only having a single unified node type would also avoid this
        RawHaalkaEl::<NodeType>::new()
            .with_component::<Style>(|style| style.display = Display::Grid)
            .child(self.raw_el.into_raw_el())
            .into_raw_el()
    }
}

impl<NodeType: Bundle + Default> PointerEventAware for Grid<NodeType> {}

impl<NodeType: Bundle + Default> Grid<NodeType> {
    pub fn row_wrap_cell_width(mut self, cell_width: f32) -> Self {
        self.raw_el = self.raw_el.with_component::<Style>(move |style| {
            style.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width);
        });
        self
    }

    pub fn row_wrap_cell_width_signal(
        mut self,
        cell_width_signal: impl Signal<Item = impl Into<Option<f32>> + Send> + Send + 'static,
    ) -> Self {
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
        self
    }

    pub fn cell<IOE: IntoOptionElement>(mut self, child_option: IOE) -> Self
    where
        <IOE::EL as RawElement>::NodeType: Bundle,
        IOE::EL: ChildProcessable,
    {
        self.raw_el = self.raw_el.child(Self::process_child(child_option));
        self
    }

    pub fn cell_signal<IOE: IntoOptionElement + 'static>(
        mut self,
        child_option: impl Signal<Item = IOE> + Send + 'static,
    ) -> Self
    where
        <IOE::EL as RawElement>::NodeType: Bundle,
        IOE::EL: ChildProcessable,
    {
        self.raw_el = self.raw_el.child_signal(child_option.map(Self::process_child));
        self
    }

    pub fn cells<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(mut self, children_options: I) -> Self
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

    pub fn cells_signal_vec<IOE: IntoOptionElement + 'static>(
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

impl<NodeType: Bundle + Default> Alignable for Grid<NodeType> {
    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Stack::<NodeType>::apply_content_alignment(style, alignment, action);
    }
}

impl<NodeType: Bundle + Default> ChildAlignable for Grid<NodeType> {
    fn update_style(style: &mut Style) {
        style.display = Display::Grid;
    }

    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Stack::<NodeType>::apply_alignment(style, alignment, action);
    }
}
