//! Stripe element with switchable layout direction.
//!
//! Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Stripe`](https://github.com/MoonZoon/MoonZoon/blob/19c6cf6b4d07cd27bee7758977ef1ea4d5b9933d/crates/zoon/src/element/stripe.rs).

use bevy_ecs::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
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
use crate::{clone_semantics_doc, impl_element_clone};

/// Direction for a [`Stripe`] element.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash)]
pub enum Direction {
    /// Vertical layout (like [`Column`](super::column::Column)).
    #[default]
    Column,
    /// Horizontal layout (like [`Row`](super::row::Row)).
    Row,
}

impl From<Direction> for FlexDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Column => FlexDirection::Column,
            Direction::Row => FlexDirection::Row,
        }
    }
}

impl From<Direction> for LayoutDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Column => LayoutDirection::Column,
            Direction::Row => LayoutDirection::Row,
        }
    }
}

/// [`Element`](super::element::Element) with children that can be laid out either horizontally or
/// vertically, with the ability to switch layout direction dynamically.
///
/// Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Stripe`](https://github.com/MoonZoon/MoonZoon/blob/19c6cf6b4d07cd27bee7758977ef1ea4d5b9933d/crates/zoon/src/element/stripe.rs).
#[doc = clone_semantics_doc!("Stripe")]
#[derive(Default)]
pub struct Stripe<NodeType> {
    builder: JonmoBuilder,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl_element_clone!("Stripe", Stripe<NodeType>, my_stripe, ".item(El::new().name(label))");

impl<NodeType: Bundle> From<JonmoBuilder> for Stripe<NodeType> {
    fn from(builder: JonmoBuilder) -> Self {
        Self {
            builder: builder
                .with_component::<Node>(|mut node| {
                    node.display = Display::Flex;
                    node.flex_direction = FlexDirection::Column;
                })
                .insert((LayoutDirection::Column, Pickable::IGNORE, Hoverable, Pressable)),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> Stripe<NodeType> {
    /// Construct a new [`Stripe`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(JonmoBuilder::from(NodeType::default()))
    }
}

impl<NodeType: Bundle> BuilderWrapper for Stripe<NodeType> {
    fn builder_mut(&mut self) -> &mut JonmoBuilder {
        &mut self.builder
    }
}

impl<NodeType: Bundle> Alignable for Stripe<NodeType> {}
impl<NodeType: Bundle> CursorOnHoverable for Stripe<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Stripe<NodeType> {}
impl<NodeType: Bundle> Nameable for Stripe<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for Stripe<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for Stripe<NodeType> {}
impl<NodeType: Bundle> UiRootable for Stripe<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Stripe<NodeType> {}

impl<NodeType: Bundle> BuilderPassThrough for Stripe<NodeType> {}

impl<NodeType: Bundle> Stripe<NodeType> {
    /// Declare a static dynamically directioned item.
    pub fn item<IOE: IntoOptionElement>(self, item_option: IOE) -> Self {
        if let Some(item) = item_option.into_option_element() {
            self.with_builder(|builder| builder.child(item.into_builder()))
        } else {
            self
        }
    }

    /// Declare a reactive dynamically directioned item. When the [`Signal`] outputs [`None`], the
    /// item is removed.
    pub fn item_signal<IOE, S>(self, item_option_signal_option: impl Into<Option<S>>) -> Self
    where
        IOE: IntoOptionElement + 'static,
        S: Signal<Item = IOE> + Send + Sync + 'static,
    {
        if let Some(item_option_signal) = item_option_signal_option.into() {
            self.with_builder(|builder| {
                builder.child_signal(
                    item_option_signal
                        .map_in(move |item_option: IOE| item_option.into_option_element().map(|el| el.into_builder())),
                )
            })
        } else {
            self
        }
    }

    /// Declare static dynamically directioned items.
    pub fn items<IOE: IntoOptionElement + 'static, I: IntoIterator<Item = IOE>>(
        self,
        items_options_option: impl Into<Option<I>>,
    ) -> Self
    where
        I::IntoIter: Send + 'static,
    {
        if let Some(items_options) = items_options_option.into() {
            self.with_builder(|builder| {
                builder.children(
                    items_options
                        .into_iter()
                        .filter_map(|item_option| item_option.into_option_element())
                        .map(|el| el.into_builder()),
                )
            })
        } else {
            self
        }
    }

    /// Declare reactive dynamically directioned items.
    pub fn items_signal_vec<IOE, S>(self, items_options_signal_vec_option: impl Into<Option<S>>) -> Self
    where
        IOE: IntoOptionElement + Clone + 'static,
        S: SignalVec<Item = IOE> + Send + Sync + 'static,
    {
        if let Some(items_options_signal_vec) = items_options_signal_vec_option.into() {
            self.with_builder(|builder| {
                builder.children_signal_vec(items_options_signal_vec.filter_map(|In(item_option): In<IOE>| {
                    item_option.into_option_element().map(|el| el.into_builder())
                }))
            })
        } else {
            self
        }
    }

    /// Set the layout direction.
    pub fn direction(self, direction: Direction) -> Self {
        let flex_direction: FlexDirection = direction.into();
        self.with_builder(|builder| {
            builder
                .with_component::<Node>(move |mut node| {
                    node.flex_direction = flex_direction;
                    node.align_items = if direction == Direction::Row {
                        AlignItems::Center
                    } else {
                        AlignItems::Default
                    };
                })
                .insert(LayoutDirection::from(direction))
        })
    }

    /// Set the layout direction reactively.
    pub fn direction_signal<S>(self, direction_signal: S) -> Self
    where
        S: Signal<Item = Direction> + Send + Sync + 'static,
    {
        self.with_builder(|builder| {
            builder.on_signal_with_entity(direction_signal.dedupe(), |mut entity_world_mut, direction| {
                if let Some(mut node) = entity_world_mut.get_mut::<Node>() {
                    node.flex_direction = direction.into();
                    node.align_items = if direction == Direction::Row {
                        AlignItems::Center
                    } else {
                        AlignItems::Default
                    };
                }
                entity_world_mut.insert(LayoutDirection::from(direction));
            })
        })
    }

    /// When the width of the stripe (in row direction) exceeds the width of its parent, wrap the
    /// stripe's children to the next line, recursively.
    pub fn multiline_row(self) -> Self {
        self.with_builder(|builder| {
            builder.with_component::<Node>(|mut node| {
                if node.flex_direction == FlexDirection::Row {
                    node.flex_wrap = FlexWrap::Wrap;
                    node.flex_basis = Val::Px(0.);
                    node.flex_grow = 1.;
                }
            })
        })
    }

    /// When the width of the stripe (in row direction) exceeds the width of its parent, wrap the
    /// stripe's children to the next line, reactively.
    pub fn multiline_row_signal<S>(self, multiline_signal: S) -> Self
    where
        S: Signal<Item = bool> + Send + Sync + 'static,
    {
        self.with_builder(|builder| {
            builder.on_signal_with_component::<Node, _, _, _>(multiline_signal.dedupe(), |mut node, multiline| {
                if node.flex_direction == FlexDirection::Row && multiline {
                    node.flex_wrap = FlexWrap::Wrap;
                    node.flex_basis = Val::Px(0.);
                    node.flex_grow = 1.;
                } else {
                    node.flex_wrap = FlexWrap::NoWrap;
                    node.flex_basis = Val::Auto;
                    node.flex_grow = 0.;
                }
            })
        })
    }
}
