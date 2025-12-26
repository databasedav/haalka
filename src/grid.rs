//! Simple grid layout model ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Grid`](https://github.com/MoonZoon/MoonZoon/blob/f8fc31065f65bdb3ab7b94faf5e3916bc5550dd9/crates/zoon/src/element/grid.rs).

use bevy_ecs::prelude::*;
use bevy_log::warn;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use bevy_utils::prelude::*;
use jonmo::{
    builder::JonmoBuilder,
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt},
};

use super::{
    align::{Alignable, LayoutDirection},
    element::{BuilderPassThrough, BuilderWrapper, IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, Hoverable, PointerEventAware, Pressable},
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::element::Element) with children aligned in a grid using a simple [`.row_wrap_cell_width`](Grid::row_wrap_cell_width) grid layout model. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/grid.rs).
///
/// # `Clone` semantics
/// This type derives `Clone`, but cloning should be treated as cloning a **build plan**, not a
/// reusable UI template.
///
/// Internally this wraps a `jonmo::builder::JonmoBuilder`. `JonmoBuilder` is `Clone`, but some of
/// its internal on-spawn hooks are **one-shot** and may be drained/consumed on the first spawn.
/// Because clones share those internal queues, spawning one clone can affect later spawns of other
/// clones.
///
/// If you need to spawn the same UI multiple times with identical initialization, prefer using a
/// factory function (e.g. `fn widget(...) -> impl Element`) that constructs a fresh `Grid` each
/// time instead of cloning a pre-built value.
#[derive(Default)]
pub struct Grid<NodeType> {
    builder: JonmoBuilder,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType> Clone for Grid<NodeType> {
    #[track_caller]
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        warn!(
            "Cloning Grid at {}. Note: Grid wraps JonmoBuilder, whose Clone shares some one-shot \
             on-spawn hook queues. Spawning one clone can affect later spawns of other clones. \
             Prefer factory functions that construct a fresh element when you need reusable UI \
             templates.",
            std::panic::Location::caller()
        );

        Self {
            builder: self.builder.clone(),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle> From<JonmoBuilder> for Grid<NodeType> {
    fn from(builder: JonmoBuilder) -> Self {
        Self {
            builder: builder
                .with_component::<Node>(|mut node| {
                    node.display = Display::Grid;
                })
                .insert((LayoutDirection::Grid, Pickable::IGNORE, Hoverable, Pressable)),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle> Grid<NodeType> {
    /// Construct a new [`Grid`] from a bundle.
    pub fn from_bundle(node_bundle: NodeType) -> Self {
        JonmoBuilder::from(node_bundle).into()
    }
}

impl<NodeType: Bundle + Default> Grid<NodeType> {
    /// Construct a new [`Grid`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from_bundle(NodeType::default())
    }
}

impl<NodeType> BuilderWrapper for Grid<NodeType> {
    fn builder_mut(&mut self) -> &mut JonmoBuilder {
        &mut self.builder
    }
}

impl<NodeType> BuilderPassThrough for Grid<NodeType> {}

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
    /// Sets the width of each grid column. The grid will automatically create as many columns
    /// as can fit within the container's width, wrapping items to new rows as needed.
    ///
    /// This uses CSS Grid's `auto-fill` behavior: `columns_per_row = floor(container_width / cell_width)`.
    ///
    /// For example, with a 300px wide container and `row_wrap_cell_width(100.0)`:
    ///
    /// ```text
    /// [  A  ][  B  ][  C  ]   <- 3 columns fit (300 / 100 = 3)
    /// [  D  ][  E  ]
    /// ```
    ///
    /// With the same container but `row_wrap_cell_width(150.0)`:
    ///
    /// ```text
    /// [  A  ][  B  ]          <- 2 columns fit (300 / 150 = 2)
    /// [  C  ][  D  ]
    /// [  E  ]
    /// ```
    ///
    /// For more complex grid layouts, modify the [`Node`] component's grid fields directly.
    ///
    /// Ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/element/grid.rs#L95).
    pub fn row_wrap_cell_width(self, cell_width_option: impl Into<Option<f32>>) -> Self {
        if let Some(cell_width) = cell_width_option.into() {
            self.with_builder(|builder| {
                builder.with_component::<Node>(move |mut node| {
                    node.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width);
                })
            })
        } else {
            self
        }
    }

    /// Reactively set the [`row_wrap_cell_width`](Self::row_wrap_cell_width).
    pub fn row_wrap_cell_width_signal<S: Signal<Item = f32> + Send + Sync + 'static>(
        self,
        cell_width_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cell_width_signal) = cell_width_signal_option.into() {
            self.with_builder(|builder| {
                builder.on_signal_with_component::<Node, _, _, _>(cell_width_signal, |mut node, cell_width| {
                    node.grid_template_columns = RepeatedGridTrack::px(GridTrackRepetition::AutoFill, cell_width)
                })
            })
        } else {
            self
        }
    }

    /// Declare a static grid child.
    pub fn cell<IOE: IntoOptionElement>(self, cell_option: IOE) -> Self {
        if let Some(cell) = cell_option.into_option_element() {
            self.with_builder(|builder| builder.child(cell.into_builder()))
        } else {
            self
        }
    }

    /// Declare a reactive grid child. When the [`Signal`] outputs [`None`], the child is
    /// removed.
    pub fn cell_signal<IOE: IntoOptionElement + 'static, S: Signal<Item = IOE> + Send + Sync + 'static>(
        self,
        cell_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cell_option_signal) = cell_option_signal_option.into() {
            self.with_builder(|builder| {
                builder.child_signal(
                    cell_option_signal
                        .map_in(move |cell_option: IOE| cell_option.into_option_element().map(|el| el.into_builder())),
                )
            })
        } else {
            self
        }
    }

    /// Declare static grid children.
    pub fn cells<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        self,
        cells_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(cells_options) = cells_options_option.into() {
            self.with_builder(|builder| {
                builder.children(
                    cells_options
                        .into_iter()
                        .filter_map(move |cell_option| cell_option.into_option_element().map(|el| el.into_builder())),
                )
            })
        } else {
            self
        }
    }

    /// Declare reactive grid children.
    pub fn cells_signal_vec<
        IOE: IntoOptionElement + Clone + 'static,
        S: SignalVec<Item = IOE> + Send + Sync + 'static,
    >(
        self,
        cells_options_signal_vec_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(cells_options_signal_vec) = cells_options_signal_vec_option.into() {
            self.with_builder(|builder| {
                builder.children_signal_vec(cells_options_signal_vec.filter_map(move |In(cell_option): In<IOE>| {
                    cell_option.into_option_element().map(|el| el.into_builder())
                }))
            })
        } else {
            self
        }
    }
}

impl<NodeType: Bundle> Alignable for Grid<NodeType> {
    fn layout_direction() -> LayoutDirection {
        LayoutDirection::Grid
    }
}
