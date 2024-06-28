use bevy::prelude::*;
use bevy_mod_picking::picking_core::Pickable;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use super::{
    align::{AddRemove, AlignHolder, Alignable, Aligner, Alignment, ChildAlignable},
    element::{GlobalEventAware, IntoOptionElement},
    pointer_event_aware::PointerEventAware,
    raw::{RawElWrapper, RawHaalkaEl},
    scrollable::Scrollable,
    sizeable::Sizeable,
    stack::Stack,
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::Element) with children aligned in a grid using a simple [`.row_wrap_cell_width`](Grid::row_wrap_cell_width) grid layout model. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon/tree/main)'s [`Grid`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/grid.rs).
pub struct Grid<NodeType> {
    raw_el: RawHaalkaEl,
    align: Option<AlignHolder>,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<NodeType> for Grid<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        Self {
            raw_el: {
                RawHaalkaEl::from(node_bundle)
                    .with_component::<Style>(|mut style| {
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
    /// Construct a new [`Grid`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the required bevy_ui node components (e.g. [`Node`], [`Style`], etc.)
    /// will not behave as expected.
    pub fn new() -> Self {
        Self::from(NodeType::default())
    }
}

impl<NodeType: Bundle> RawElWrapper for Grid<NodeType> {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.raw_el
    }

    fn into_raw_el(self) -> RawHaalkaEl {
        // TODO: why won't grid_template_columns work without a grid wrapper ?
        RawHaalkaEl::from(NodeBundle::default())
            .with_component::<Style>(|mut style| style.display = Display::Grid)
            .child(self.raw_el)
    }
}

impl<NodeType: Bundle> PointerEventAware for Grid<NodeType> {}
impl<NodeType: Bundle> Scrollable for Grid<NodeType> {}
impl<NodeType: Bundle> Sizeable for Grid<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Grid<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Grid<NodeType> {}

/// Must substract this from the total row width of a [`Grid`] due to [float precision shenanigans](https://github.com/bevyengine/bevy/issues/12152). See an example usage in the [snake example](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/snake.rs#L112).
pub const GRID_TRACK_FLOAT_PRECISION_SLACK: f32 = 0.0001;

impl<NodeType: Bundle> Grid<NodeType> {
    /// Simple grid layout model [ported from MooonZoon](https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/grid.rs#L95).
    /// The `cell_width` passed is simply the width all cells must occupy without overflowing their
    /// parent; if a cell with said width does overflow its parent, it will wrap around to the next
    /// row.
    ///
    /// For example, let's say our grid items where the letters A to E, where each letter occupies 1
    /// unit of width. Then a `row_wrap_cell_width` of 3 would result in the following grid:
    ///
    /// ```text
    /// A B C
    /// D E
    /// ```
    ///
    /// `row_wrap_cell_width` of 2:
    ///
    /// ```text
    /// A B
    /// C D
    /// E
    /// ```
    ///
    /// and 1:
    /// ```text
    /// A
    /// B
    /// C
    /// D
    /// E
    /// ```
    ///
    /// While this grid layout definition is not nearly as rich as the [CSS grid layout](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout),
    /// it may suffice for one's needs. If not, one can always use bevy_ui's CSS grid API directly
    /// by modifiying the appropriate fields on any UI node's [`Style`] [`Component`].
    pub fn row_wrap_cell_width(mut self, cell_width_option: impl Into<Option<f32>>) -> Self {
        if let Some(cell_width) = cell_width_option.into() {
            self.raw_el = self.raw_el.with_component::<Style>(move |mut style| {
                style.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width);
            });
        }
        self
    }

    /// Reactively set the [`row_wrap_cell_width`](Self::row_wrap_cell_width).
    pub fn row_wrap_cell_width_signal<S: Signal<Item = f32> + Send + 'static>(
        mut self,
        cell_width_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cell_width_signal) = cell_width_signal_option.into() {
            self.raw_el = self.raw_el.on_signal_with_component::<f32, Style>(
                cell_width_signal.map(|cell_width| cell_width.into()),
                |mut style, cell_width| {
                    style.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width)
                },
            );
        }
        self
    }

    /// Declare a static grid child.
    pub fn cell<IOE: IntoOptionElement>(mut self, cell_option: IOE) -> Self {
        let apply_alignment = self.apply_alignment_wrapper();
        self.raw_el = self.raw_el.child(
            cell_option
                .into_option_element()
                .map(|cell| Self::align_child(cell, apply_alignment)),
        );
        self
    }

    /// Declare a reactive grid child. When the [`Signal`] outputs [`None`], the child is
    /// removed.
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

    /// Declare static grid children.
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

    /// Declare reactive grid children.
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
    fn aligner(&mut self) -> Option<Aligner> {
        Some(Aligner::Grid)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Stack::<NodeType>::apply_content_alignment(style, alignment, action);
    }
}

impl<NodeType: Bundle> ChildAlignable for Grid<NodeType> {
    fn update_style(mut style: Mut<Style>) {
        style.display = Display::Grid;
    }

    fn apply_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        Stack::<NodeType>::apply_alignment(style, alignment, action);
    }
}
