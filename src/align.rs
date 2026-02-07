//! Simple alignment semantics ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`align`](https://github.com/MoonZoon/MoonZoon/blob/19c6cf6b4d07cd27bee7758977ef1ea4d5b9933d/crates/zoon/src/style/align.rs) and [`align_content`](https://github.com/MoonZoon/MoonZoon/blob/19c6cf6b4d07cd27bee7758977ef1ea4d5b9933d/crates/zoon/src/style/align_content.rs)
//!
//! An [`Element`](`super::element::Element`) can be aligned in nine different areas in relation to
//! its parent: top left, top center, top right, center left, center, center right, bottom left,
//! bottom center, and bottom right. This provides a simple and clear way to declare alignment as
//! a thin layer on top of bevy_ui's flexbox and grid implementations.
//!
//! [`Align`]s can be specified on individual elements using [`.align`](`Alignable::align`) and
//! [`.align_signal`](`Alignable::align_signal`) or to all children using
//! [`.align_content`](`Alignable::align_content`) and
//! [`.align_content_signal`](`Alignable::align_content_signal`). See the [align](https://github.com/databasedav/haalka/blob/main/examples/align.rs)
//! example for how each [`Align`] behaves for each built-in alignable type: [`El`](super::el::El),
//! [`Column`](super::column::Column), [`Row`](super::row::Row), [`Stripe`](super::stripe::Stripe),
//! [`Stack`](super::stack::Stack), and [`Grid`](super::grid::Grid).

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

/// Component for self-alignment of an element within its parent. Applied to children and processed
/// based on the parent's [`AlignmentHandler`].
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[component(on_remove = on_alignment_remove)]
pub struct Alignment {
    /// Horizontal alignment.
    pub x: AlignX,
    /// Vertical alignment.
    pub y: AlignY,
}

/// Component for content alignment (how a parent aligns its children). Applied to parents to
/// control default alignment of all children.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[component(on_remove = on_content_alignment_remove)]
pub struct ContentAlignment {
    /// Horizontal content alignment.
    pub x: AlignX,
    /// Vertical content alignment.
    pub y: AlignY,
}

/// Function signature for applying self-alignment to a child node.
pub type ApplyAlignmentFn = fn(&mut Node, &Alignment);

/// Function signature for applying content alignment to a parent node.
pub type ApplyContentAlignmentFn = fn(&mut Node, &ContentAlignment);

/// Function signature for resetting alignment styles when the component is removed.
pub type ResetAlignmentFn = fn(&mut Node);

/// Handler component that defines how self-alignment is applied to a child node.
///
/// This is the low-level component that drives alignment behavior. Users typically
/// don't interact with this directly, instead use [`LayoutDirection`] which
/// automatically installs the appropriate handlers.
#[derive(Component, Clone, Copy)]
pub struct AlignmentHandler {
    /// Function to apply alignment to the node.
    pub apply: ApplyAlignmentFn,
    /// Function to reset alignment styles when [`Alignment`] is removed.
    pub reset: ResetAlignmentFn,
}

/// Handler component that defines how content alignment is applied to a parent node.
///
/// This is the low-level component that drives content alignment behavior. Users typically
/// don't interact with this directly, instead use [`LayoutDirection`] which
/// automatically installs the appropriate handlers.
#[derive(Component, Clone, Copy)]
pub struct ContentAlignmentHandler {
    /// Function to apply content alignment to the node.
    pub apply: ApplyContentAlignmentFn,
    /// Function to reset content alignment styles when [`ContentAlignment`] is removed.
    pub reset: ResetAlignmentFn,
}

/// Simple, [haalka](crate)-managed layout direction options for container elements.
///
/// When inserted, it automatically installs the appropriate [`AlignmentHandler`] and
/// [`ContentAlignmentHandler`] components via a component hook.
///
/// For custom alignment behavior, either:
/// - Insert handlers directly without using this enum
/// - Insert this enum, then replace the desired handlers with custom ones
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug, Hash)]
#[component(on_insert = on_layout_direction_insert)]
pub enum LayoutDirection {
    /// Vertical stacking layout (like [`Column`](super::column::Column)).
    ///
    /// - X-axis alignment uses `align_self`
    /// - Y-axis alignment uses `margin.top`/`margin.bottom`
    /// - Content X uses `align_items`, content Y uses `justify_content`
    #[default]
    Column,
    /// Horizontal stacking layout (like [`Row`](super::row::Row)).
    ///
    /// - X-axis alignment uses `margin.left`/`margin.right`
    /// - Y-axis alignment uses `align_self`
    /// - Content X uses `justify_content`, content Y uses `align_items`
    Row,
    /// Grid/Stack layout (children can overlap in same cell).
    ///
    /// - X-axis alignment uses `justify_self`
    /// - Y-axis alignment uses `align_self`
    /// - Content uses same semantics as row
    Grid,
}

/// Hook that installs appropriate handlers when [`LayoutDirection`] is inserted.
fn on_layout_direction_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    // Reset children's alignment using the previous handler before installing the new one
    if let Some(old_handler) = world.get::<AlignmentHandler>(entity).copied() {
        let children: Vec<Entity> = world
            .get::<Children>(entity)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        for child in children {
            if world.get::<Alignment>(child).is_some()
                && let Some(mut node) = world.get_mut::<Node>(child)
            {
                (old_handler.reset)(&mut node);
            }
        }
    }

    let direction = world.get::<LayoutDirection>(entity).copied().unwrap();

    let (alignment_handler, content_alignment_handler) = match direction {
        LayoutDirection::Column => (
            AlignmentHandler {
                apply: column::apply_alignment,
                reset: column::reset_alignment,
            },
            ContentAlignmentHandler {
                apply: column::apply_content_alignment,
                reset: column::reset_content_alignment,
            },
        ),
        LayoutDirection::Row => (
            AlignmentHandler {
                apply: row::apply_alignment,
                reset: row::reset_alignment,
            },
            ContentAlignmentHandler {
                apply: row::apply_content_alignment,
                reset: row::reset_content_alignment,
            },
        ),
        LayoutDirection::Grid => (
            AlignmentHandler {
                apply: grid::apply_alignment,
                reset: grid::reset_alignment,
            },
            ContentAlignmentHandler {
                // Grid uses row semantics for content alignment
                apply: row::apply_content_alignment,
                reset: row::reset_content_alignment,
            },
        ),
    };

    world
        .commands()
        .entity(entity)
        .insert((alignment_handler, content_alignment_handler));
}

/// Composable alignment builder. Used with [`.align`](`Alignable::align`) and related methods.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
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
///
/// # Requirements
/// For alignment methods to function:
/// - **Self-alignment** ([`align`](Alignable::align), [`align_signal`](Alignable::align_signal)):
///   The **parent** entity must have an [`AlignmentHandler`] component.
/// - **Content alignment** ([`align_content`](Alignable::align_content),
///   [`align_content_signal`](Alignable::align_content_signal)): The entity itself must have a
///   [`ContentAlignmentHandler`] component.
///
/// The built-in element types ([`El`](super::el::El), [`Column`](super::column::Column),
/// [`Row`](super::row::Row), [`Stack`](super::stack::Stack), [`Grid`](super::grid::Grid))
/// automatically insert the appropriate handlers via their [`LayoutDirection`] component. For
/// custom elements, either insert [`LayoutDirection`] or manually insert the handler components.
pub trait Alignable: BuilderWrapper + Sized {
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
                builder.component_signal(
                    align_option_signal.map_in(|align_option| align_option.map(|align| align.to_alignment())),
                )
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
                    align_option_signal.map_in(|align_option| align_option.map(|align| align.to_content_alignment())),
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

/// System that applies self-alignment based on parent's alignment handler.
#[allow(clippy::type_complexity)]
fn apply_self_alignment(
    mut data: Query<(&Alignment, &ChildOf, &mut Node), Or<(Changed<Alignment>, Added<Alignment>, Added<ChildOf>)>>,
    handlers: Query<&AlignmentHandler>,
) {
    for (alignment, child_of, mut node) in &mut data {
        if let Ok(handler) = handlers.get(child_of.parent()) {
            (handler.apply)(&mut node, alignment);
        }
    }
}

/// System that re-applies self-alignment when parent's alignment handler changes.
fn apply_self_alignment_on_parent_change(
    mut children_query: Query<(&Alignment, &mut Node)>,
    changed_parents: Query<(&AlignmentHandler, &Children), Changed<AlignmentHandler>>,
) {
    for (handler, children) in &changed_parents {
        for &child in children {
            if let Ok((alignment, mut node)) = children_query.get_mut(child) {
                (handler.apply)(&mut node, alignment);
            }
        }
    }
}

/// Hook called when Alignment component is removed, resetting node properties.
fn on_alignment_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let handler = world
        .get::<ChildOf>(entity)
        .and_then(|child_of| world.get::<AlignmentHandler>(child_of.parent()).copied());

    if let (Some(handler), Some(mut node)) = (handler, world.get_mut::<Node>(entity)) {
        (handler.reset)(&mut node);
    }
}

/// System that applies content alignment to parent nodes.
#[allow(clippy::type_complexity)]
fn apply_content_alignment(
    mut data: Query<
        (&ContentAlignment, &ContentAlignmentHandler, &mut Node),
        Or<(
            Changed<ContentAlignment>,
            Added<ContentAlignment>,
            Changed<ContentAlignmentHandler>,
        )>,
    >,
) {
    for (content_alignment, handler, mut node) in &mut data {
        (handler.apply)(&mut node, content_alignment);
    }
}

/// Hook called when ContentAlignment component is removed, resets node properties.
fn on_content_alignment_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let handler = world.get::<ContentAlignmentHandler>(entity).copied();

    if let (Some(handler), Some(mut node)) = (handler, world.get_mut::<Node>(entity)) {
        (handler.reset)(&mut node);
    }
}

/// Column alignment implementations.
pub mod column {
    use super::*;

    /// Apply column-style self-alignment to a child node.
    ///
    /// X-axis uses `align_self`, Y-axis uses `margin.top`/`margin.bottom`.
    pub fn apply_alignment(node: &mut Node, alignment: &Alignment) {
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

    /// Apply column-style content alignment to a parent node.
    ///
    /// X-axis uses `align_items`, Y-axis uses `justify_content`.
    pub fn apply_content_alignment(node: &mut Node, content_alignment: &ContentAlignment) {
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

    /// Reset column-style alignment on a child node.
    pub fn reset_alignment(node: &mut Node) {
        node.align_self = AlignSelf::DEFAULT;
        node.margin.top = Val::ZERO;
        node.margin.bottom = Val::ZERO;
    }

    /// Reset column-style content alignment on a parent node.
    pub fn reset_content_alignment(node: &mut Node) {
        node.align_items = AlignItems::DEFAULT;
        node.justify_content = JustifyContent::DEFAULT;
    }
}

/// Row alignment implementations.
pub mod row {
    use super::*;

    /// Apply row-style self-alignment to a child node.
    ///
    /// X-axis uses `margin.left`/`margin.right`, Y-axis uses `align_self`.
    pub fn apply_alignment(node: &mut Node, alignment: &Alignment) {
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

    /// Apply row-style content alignment to a parent node.
    ///
    /// X-axis uses `justify_content`, Y-axis uses `align_items`.
    pub fn apply_content_alignment(node: &mut Node, content_alignment: &ContentAlignment) {
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

    /// Reset row-style alignment on a child node.
    pub fn reset_alignment(node: &mut Node) {
        node.margin.left = Val::ZERO;
        node.margin.right = Val::ZERO;
        node.align_self = AlignSelf::DEFAULT;
    }

    /// Reset row-style content alignment on a parent node.
    pub fn reset_content_alignment(node: &mut Node) {
        node.justify_content = JustifyContent::DEFAULT;
        node.align_items = AlignItems::DEFAULT;
    }
}

/// Grid alignment implementations.
pub mod grid {
    use super::*;

    /// Apply grid-style self-alignment to a child node.
    ///
    /// X-axis uses `justify_self`, Y-axis uses `align_self`.
    pub fn apply_alignment(node: &mut Node, alignment: &Alignment) {
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

    /// Reset grid-style alignment on a child node.
    pub fn reset_alignment(node: &mut Node) {
        node.justify_self = JustifySelf::DEFAULT;
        node.align_self = AlignSelf::DEFAULT;
    }

    // Grid uses row semantics for content alignment, so no separate functions needed.
}
