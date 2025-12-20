//! Alignment system using marker components for a more Bevy-native approach.
//!
//! An [`Element`](`super::element::Element`) can be aligned in nine different areas in relation to
//! its parent: top left, top center, top right, center left, center, center right, bottom left,
//! bottom center, and bottom right. This provides a simple and clear way to declare alignment as
//! a thin layer on top of bevy_ui's flexbox and grid implementations.
//!
//! The alignment system uses marker components ([`Alignment`] and [`ContentAlignment`]) that are
//! processed by Bevy systems based on the parent's [`LayoutDirection`]. This allows for flexible
//! layouts where children automatically adapt when a parent's direction changes (similar to
//! MoonZoon's Stripe element).
//!
//! [`Align`]s can be specified on individual elements using [`.align`](`Alignable::align`) and
//! [`.align_signal`](`Alignable::align_signal`) or to all children using
//! [`.align_content`](`Alignable::align_content`) and
//! [`.align_content_signal`](`Alignable::align_content_signal`). See the [align](https://github.com/databasedav/haalka/blob/main/examples/align.rs)
//! example for how each [`Align`] behaves for each built-in alignable type: [`El`], [`Column`],
//! [`Row`], [`Stack`], and [`Grid`].

use bevy_app::prelude::*;
use bevy_ecs::{lifecycle::HookContext, prelude::*, world::DeferredWorld};
use bevy_ui::prelude::*;
use jonmo::{
    SignalProcessing,
    signal::{Signal, SignalExt},
};

use super::element::BuilderWrapper;

/// Horizontal alignment axis.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Hash)]
pub enum AlignX {
    /// No horizontal alignment constraint (use default layout behavior).
    #[default]
    None,
    /// Align to the left.
    Left,
    /// Align to the horizontal center.
    Center,
    /// Align to the right.
    Right,
}

/// Vertical alignment axis.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Hash)]
pub enum AlignY {
    /// No vertical alignment constraint (use default layout behavior).
    #[default]
    None,
    /// Align to the top.
    Top,
    /// Align to the vertical center.
    Center,
    /// Align to the bottom.
    Bottom,
}

/// Marker component for self-alignment of an element within its parent.
/// Applied to children and processed based on the parent's [`LayoutDirection`].
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[component(on_remove = on_alignment_remove)]
pub struct Alignment {
    /// Horizontal alignment.
    pub x: AlignX,
    /// Vertical alignment.
    pub y: AlignY,
}

/// Marker component for content alignment (how a parent aligns its children).
/// Applied to parents to control default alignment of all children.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[component(on_remove = on_content_alignment_remove)]
pub struct ContentAlignment {
    /// Horizontal content alignment.
    pub x: AlignX,
    /// Vertical content alignment.
    pub y: AlignY,
}

/// The layout direction of a container element.
/// Determines how child alignments are interpreted.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum LayoutDirection {
    /// Vertical stacking (like [`Column`]).
    #[default]
    Column,
    /// Horizontal stacking (like [`Row`]).
    Row,
    /// Grid/Stack layout (children overlap in same cell).
    Grid,
}

/// Composable alignment builder. Used with [`.align`](`Alignable::align`) and related methods.
#[derive(Clone, Copy, Default, Debug)]
pub struct Align {
    x: AlignX,
    y: AlignY,
}

impl Align {
    /// Create a new empty alignment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Center on both axes.
    pub fn center() -> Self {
        Self {
            x: AlignX::Center,
            y: AlignY::Center,
        }
    }

    /// Center horizontally.
    pub fn center_x(mut self) -> Self {
        self.x = AlignX::Center;
        self
    }

    /// Center vertically.
    pub fn center_y(mut self) -> Self {
        self.y = AlignY::Center;
        self
    }

    /// Align to top.
    pub fn top(mut self) -> Self {
        self.y = AlignY::Top;
        self
    }

    /// Align to bottom.
    pub fn bottom(mut self) -> Self {
        self.y = AlignY::Bottom;
        self
    }

    /// Align to left.
    pub fn left(mut self) -> Self {
        self.x = AlignX::Left;
        self
    }

    /// Align to right.
    pub fn right(mut self) -> Self {
        self.x = AlignX::Right;
        self
    }

    /// Convert to the marker component representation.
    fn to_alignment(self) -> Alignment {
        Alignment { x: self.x, y: self.y }
    }

    /// Convert to the content alignment marker component representation.
    fn to_content_alignment(self) -> ContentAlignment {
        ContentAlignment { x: self.x, y: self.y }
    }
}

/// Trait for elements that can be aligned and can align their content.
pub trait Alignable: BuilderWrapper + Sized {
    /// Get the layout direction for this element type.
    fn layout_direction() -> LayoutDirection;

    /// Statically align this element within its parent.
    fn align(self, align_option: impl Into<Option<Align>>) -> Self {
        if let Some(align) = align_option.into() {
            let alignment = align.to_alignment();
            self.with_builder(|builder| builder.insert(alignment))
        } else {
            self
        }
    }

    /// Reactively align this element within its parent.
    fn align_signal<S>(self, align_option_signal_option: impl Into<Option<S>>) -> Self
    where
        S: Signal<Item = Option<Align>> + Send + Sync + 'static,
    {
        if let Some(align_option_signal) = align_option_signal_option.into() {
            self.with_builder(|builder| {
                builder.component_signal(align_option_signal.map_in(|opt: Option<Align>| opt.map(|a| a.to_alignment())))
            })
        } else {
            self
        }
    }

    /// Statically align the children of this element.
    fn align_content(self, align_option: impl Into<Option<Align>>) -> Self {
        if let Some(align) = align_option.into() {
            let content_alignment = align.to_content_alignment();
            self.with_builder(|builder| builder.insert(content_alignment))
        } else {
            self
        }
    }

    /// Reactively align the children of this element.
    fn align_content_signal<S>(self, align_option_signal_option: impl Into<Option<S>>) -> Self
    where
        S: Signal<Item = Option<Align>> + Send + Sync + 'static,
    {
        if let Some(align_option_signal) = align_option_signal_option.into() {
            self.with_builder(|builder| {
                builder.component_signal(
                    align_option_signal.map_in(|opt: Option<Align>| opt.map(|a| a.to_content_alignment())),
                )
            })
        } else {
            self
        }
    }
}

/// Plugin that adds the alignment systems.
pub fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            apply_self_alignment,
            apply_self_alignment_on_parent_change,
            apply_content_alignment,
        )
            .after(SignalProcessing),
    );
}

/// System that applies self-alignment based on parent's layout direction.
fn apply_self_alignment(
    mut data: Query<(&Alignment, &ChildOf, &mut Node), Or<(Changed<Alignment>, Added<Alignment>, Added<ChildOf>)>>,
    layout_directions: Query<&LayoutDirection>,
) {
    for (alignment, child_of, mut node) in &mut data {
        let direction = layout_directions.get(child_of.parent()).copied().unwrap_or_default();
        apply_alignment_to_node(&mut node, alignment, direction);
    }
}

/// System that re-applies self-alignment when parent's layout direction changes.
fn apply_self_alignment_on_parent_change(
    mut children_query: Query<(&Alignment, &mut Node)>,
    changed_parents: Query<(&LayoutDirection, &Children), Changed<LayoutDirection>>,
) {
    for (direction, children) in &changed_parents {
        for &child in children {
            if let Ok((alignment, mut node)) = children_query.get_mut(child) {
                apply_alignment_to_node(&mut node, alignment, *direction);
            }
        }
    }
}

/// Hook called when Alignment component is removed - resets node styles.
fn on_alignment_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let direction = world
        .get::<ChildOf>(entity)
        .and_then(|child_of| world.get::<LayoutDirection>(child_of.parent()).copied())
        .unwrap_or_default();

    if let Some(mut node) = world.get_mut::<Node>(entity) {
        match direction {
            LayoutDirection::Column => {
                // Column child alignment uses: align_self (X) + margin.top/bottom (Y)
                node.align_self = AlignSelf::DEFAULT;
                node.margin.top = Val::ZERO;
                node.margin.bottom = Val::ZERO;
            }
            LayoutDirection::Row => {
                // Row child alignment uses: margin.left/right (X) + align_self (Y)
                node.margin.left = Val::ZERO;
                node.margin.right = Val::ZERO;
                node.align_self = AlignSelf::DEFAULT;
            }
            LayoutDirection::Grid => {
                // Grid/Stack child alignment uses: justify_self (X) + align_self (Y)
                node.justify_self = JustifySelf::DEFAULT;
                node.align_self = AlignSelf::DEFAULT;
            }
        }
    }
}

/// System that applies content alignment to parent nodes.
fn apply_content_alignment(
    mut data: Query<
        (&ContentAlignment, &LayoutDirection, &mut Node),
        Or<(Changed<ContentAlignment>, Added<ContentAlignment>)>,
    >,
) {
    for (content_alignment, direction, mut node) in &mut data {
        apply_content_alignment_to_node(&mut node, content_alignment, *direction);
    }
}

/// Hook called when ContentAlignment component is removed - resets node styles.
fn on_content_alignment_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let direction = world.get::<LayoutDirection>(entity).copied().unwrap_or_default();
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        match direction {
            LayoutDirection::Column => {
                // Column content alignment uses: align_items (X) + justify_content (Y)
                node.align_items = AlignItems::DEFAULT;
                node.justify_content = JustifyContent::DEFAULT;
            }
            LayoutDirection::Row | LayoutDirection::Grid => {
                // Row content alignment uses: justify_content (X) + align_items (Y)
                // Origin Stack/Grid content alignment delegates to Row.
                node.justify_content = JustifyContent::DEFAULT;
                node.align_items = AlignItems::DEFAULT;
            }
        }
    }
}

/// Apply self-alignment to a node based on parent direction.
fn apply_alignment_to_node(node: &mut Node, alignment: &Alignment, direction: LayoutDirection) {
    match direction {
        LayoutDirection::Column => {
            // In a column, X-axis uses align_self, Y-axis uses margin
            node.align_self = match alignment.x {
                AlignX::None => AlignSelf::DEFAULT,
                AlignX::Left => AlignSelf::Start,
                AlignX::Center => AlignSelf::Center,
                AlignX::Right => AlignSelf::End,
            };

            (node.margin.top, node.margin.bottom) = match alignment.y {
                AlignY::None => (Val::ZERO, Val::ZERO),
                AlignY::Top => (Val::ZERO, Val::Auto),
                AlignY::Center => (Val::Auto, Val::Auto),
                AlignY::Bottom => (Val::Auto, Val::ZERO),
            };
        }
        LayoutDirection::Row => {
            // In a row, X-axis uses margin, Y-axis uses align_self
            (node.margin.left, node.margin.right) = match alignment.x {
                AlignX::None => (Val::ZERO, Val::ZERO),
                AlignX::Left => (Val::ZERO, Val::Auto),
                AlignX::Center => (Val::Auto, Val::Auto),
                AlignX::Right => (Val::Auto, Val::ZERO),
            };

            node.align_self = match alignment.y {
                AlignY::None => AlignSelf::DEFAULT,
                AlignY::Top => AlignSelf::Start,
                AlignY::Center => AlignSelf::Center,
                AlignY::Bottom => AlignSelf::End,
            };
        }
        LayoutDirection::Grid => {
            // In a grid/stack, self alignment uses justify_self (X) and align_self (Y)
            // (matches origin Stack/Grid child alignment behavior).
            node.justify_self = match alignment.x {
                AlignX::None => JustifySelf::DEFAULT,
                AlignX::Left => JustifySelf::Start,
                AlignX::Center => JustifySelf::Center,
                AlignX::Right => JustifySelf::End,
            };
            node.align_self = match alignment.y {
                AlignY::None => AlignSelf::DEFAULT,
                AlignY::Top => AlignSelf::Start,
                AlignY::Center => AlignSelf::Center,
                AlignY::Bottom => AlignSelf::End,
            };
        }
    }
}

/// Apply content alignment to a parent node based on its direction.
fn apply_content_alignment_to_node(node: &mut Node, content_alignment: &ContentAlignment, direction: LayoutDirection) {
    match direction {
        LayoutDirection::Column => {
            // In a column, X-axis uses align_items, Y-axis uses justify_content
            node.align_items = match content_alignment.x {
                AlignX::None => AlignItems::DEFAULT,
                AlignX::Left => AlignItems::Start,
                AlignX::Center => AlignItems::Center,
                AlignX::Right => AlignItems::End,
            };
            node.justify_content = match content_alignment.y {
                AlignY::None => JustifyContent::DEFAULT,
                AlignY::Top => JustifyContent::Start,
                AlignY::Center => JustifyContent::Center,
                AlignY::Bottom => JustifyContent::End,
            };
        }
        LayoutDirection::Row => {
            // In a row, X-axis uses justify_content, Y-axis uses align_items
            node.justify_content = match content_alignment.x {
                AlignX::None => JustifyContent::DEFAULT,
                AlignX::Left => JustifyContent::Start,
                AlignX::Center => JustifyContent::Center,
                AlignX::Right => JustifyContent::End,
            };
            node.align_items = match content_alignment.y {
                AlignY::None => AlignItems::DEFAULT,
                AlignY::Top => AlignItems::Start,
                AlignY::Center => AlignItems::Center,
                AlignY::Bottom => AlignItems::End,
            };
        }
        LayoutDirection::Grid => {
            // Origin Stack/Grid content alignment delegates to Row semantics.
            node.justify_content = match content_alignment.x {
                AlignX::None => JustifyContent::DEFAULT,
                AlignX::Left => JustifyContent::Start,
                AlignX::Center => JustifyContent::Center,
                AlignX::Right => JustifyContent::End,
            };
            node.align_items = match content_alignment.y {
                AlignY::None => AlignItems::DEFAULT,
                AlignY::Top => AlignItems::Start,
                AlignY::Center => AlignItems::Center,
                AlignY::Bottom => AlignItems::End,
            };
        }
    }
}
