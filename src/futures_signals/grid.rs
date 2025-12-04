//! Simple grid layout model ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Grid`](https://github.com/MoonZoon/MoonZoon/blob/f8fc31065f65bdb3ab7b94faf5e3916bc5550dd9/crates/zoon/src/element/grid.rs).

use bevy_ecs::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use bevy_utils::prelude::*;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use super::{
    align::{Alignable, LayoutDirection},
    element::{IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, PointerEventAware},
    raw::{RawElWrapper, RawHaalkaEl},
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::element::Element) with children aligned in a grid using a simple [`.row_wrap_cell_width`](Grid::row_wrap_cell_width) grid layout model. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Grid`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/grid.rs).
#[derive(Default)]
pub struct Grid<NodeType> {
    raw_el: RawHaalkaEl,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<RawHaalkaEl> for Grid<NodeType> {
    fn from(value: RawHaalkaEl) -> Self {
        Self {
            raw_el: value
                .with_component::<Node>(|mut node| {
                    node.display = Display::Grid;
                })
                .insert(Pickable::IGNORE)
                .insert(LayoutDirection::Grid),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle> From<NodeType> for Grid<NodeType> {
    fn from(node_bundle: NodeType) -> Self {
        RawHaalkaEl::from(node_bundle).into()
    }
}

impl<NodeType: Bundle + Default> Grid<NodeType> {
    /// Construct a new [`Grid`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
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
        RawHaalkaEl::from(Node {
            display: Display::Grid,
            ..default()
        })
        .child(self.raw_el)
    }
}

impl<NodeType: Bundle> CursorOnHoverable for Grid<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Grid<NodeType> {}
impl<NodeType: Bundle> Nameable for Grid<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for Grid<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for Grid<NodeType> {}
impl<NodeType: Bundle> UiRootable for Grid<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Grid<NodeType> {}

/// Must substract this from the total row width of a [`Grid`] due to [float precision shenanigans](https://github.com/bevyengine/bevy/issues/12152). See an example usage in the [snake example](https://github.com/databasedav/haalka/blob/e12350c55d7aace07bc27787989c79d5a4e064e5/examples/snake.rs#L112).
pub const GRID_TRACK_FLOAT_PRECISION_SLACK: f32 = 0.001;

impl<NodeType: Bundle> Grid<NodeType> {
    /// Simple grid layout model [ported from MoonZoon](https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/grid.rs#L95).
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
    /// by modifiying the appropriate fields on any UI node's [`Node`] [`Component`].
    pub fn row_wrap_cell_width(mut self, cell_width_option: impl Into<Option<f32>>) -> Self {
        if let Some(cell_width) = cell_width_option.into() {
            self.raw_el = self.raw_el.with_component::<Node>(move |mut node| {
                node.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width);
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
            self.raw_el =
                self.raw_el
                    .on_signal_with_component::<f32, Node>(cell_width_signal, |mut node, cell_width| {
                        node.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width)
                    });
        }
        self
    }

    /// Declare a static grid child.
    pub fn cell<IOE: IntoOptionElement>(mut self, cell_option: IOE) -> Self {
        self.raw_el = self.raw_el.child(cell_option.into_option_element());
        self
    }

    /// Declare a reactive grid child. When the [`Signal`] outputs [`None`], the child is
    /// removed.
    pub fn cell_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + 'static>(
        mut self,
        cell_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cell_option_signal) = cell_option_signal_option.into() {
            self.raw_el = self.raw_el.child_signal(cell_option_signal.map(move |cell_option| {
                cell_option.into_option_element()
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
            self.raw_el = self.raw_el.children(cells_options.into_iter().map(move |cell_option| {
                cell_option.into_option_element()
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
            self.raw_el = self
                .raw_el
                .children_signal_vec(cells_options_signal_vec.map(move |cell_option| {
                    cell_option.into_option_element()
                }));
        }
        self
    }
}

impl<NodeType: Bundle> Alignable for Grid<NodeType> {
    fn layout_direction() -> LayoutDirection {
        LayoutDirection::Grid
    }
}

