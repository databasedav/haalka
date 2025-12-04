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
    element::{BuilderWrapper, IntoOptionElement, Nameable, UiRootable},
    global_event_aware::GlobalEventAware,
    mouse_wheel_scrollable::MouseWheelScrollable,
    pointer_event_aware::{CursorOnHoverable, PointerEventAware},
    viewport_mutable::ViewportMutable,
};

/// [`Element`](super::element::Element) with horizontally stacked children. Port of [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`Row`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/element/row.rs).
#[derive(Default)]
pub struct Row<NodeType> {
    builder: JonmoBuilder,
    _node_type: std::marker::PhantomData<NodeType>,
}

impl<NodeType: Bundle> From<JonmoBuilder> for Row<NodeType> {
    fn from(builder: JonmoBuilder) -> Self {
        Self {
            builder: builder
                .with_component::<Node>(|mut node| {
                    node.display = Display::Flex;
                    node.flex_direction = FlexDirection::Row;
                    node.align_items = AlignItems::Center;
                })
                .insert(Pickable::IGNORE)
                .insert(LayoutDirection::Row),
            _node_type: std::marker::PhantomData,
        }
    }
}

impl<NodeType: Bundle + Default> Row<NodeType> {
    /// Construct a new [`Row`] from a [`Bundle`] with a [`Default`] implementation.
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn new() -> Self {
        Self::from(JonmoBuilder::from(NodeType::default()))
    }

    /// Construct a new [`Row`] from a [`Bundle`].
    ///
    /// # Notes
    /// [`Bundle`]s without the [`Node`] component will not behave as expected.
    pub fn from_bundle(node_bundle: NodeType) -> Self {
        Self::from(JonmoBuilder::from(node_bundle))
    }
}

impl<NodeType: Bundle> BuilderWrapper for Row<NodeType> {
    fn builder_mut(&mut self) -> &mut JonmoBuilder {
        &mut self.builder
    }
}

impl<NodeType: Bundle> CursorOnHoverable for Row<NodeType> {}
impl<NodeType: Bundle> GlobalEventAware for Row<NodeType> {}
impl<NodeType: Bundle> Nameable for Row<NodeType> {}
impl<NodeType: Bundle> PointerEventAware for Row<NodeType> {}
impl<NodeType: Bundle> MouseWheelScrollable for Row<NodeType> {}
impl<NodeType: Bundle> UiRootable for Row<NodeType> {}
impl<NodeType: Bundle> ViewportMutable for Row<NodeType> {}

impl<NodeType: Bundle> Row<NodeType> {
    /// Declare a static horizontally stacked child.
    pub fn item<IOE: IntoOptionElement>(self, item_option: IOE) -> Self {
        if let Some(item) = item_option.into_option_element() {
            self.with_builder(|builder| builder.child(item.into_builder()))
        } else {
            self
        }
    }

    /// Declare a reactive horizontally stacked child. When the [`Signal`] outputs [`None`], the
    /// child is removed.
    pub fn item_signal<IOE, S>(self, item_option_signal_option: impl Into<Option<S>>) -> Self
    where
        IOE: IntoOptionElement + 'static,
        S: Signal<Item = IOE> + Send + Sync + 'static,
    {
        if let Some(item_option_signal) = item_option_signal_option.into() {
            self.with_builder(|builder| {
                builder.child_signal(item_option_signal.map_in(move |item_option: IOE| {
                    item_option.into_option_element().map(|el| el.into_builder())
                }))
            })
        } else {
            self
        }
    }

    /// Declare static horizontally stacked children.
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

    /// Declare reactive horizontally stacked children.
    pub fn items_signal_vec<IOE, S>(self, items_options_signal_vec_option: impl Into<Option<S>>) -> Self
    where
        IOE: IntoOptionElement + Clone + 'static,
        S: SignalVec<Item = IOE> + Send + Sync + 'static,
    {
        if let Some(items_options_signal_vec) = items_options_signal_vec_option.into() {
            self.with_builder(|builder| {
                builder.children_signal_vec(
                    items_options_signal_vec
                        .filter_map(|In(item_option): In<IOE>| {
                            item_option.into_option_element().map(|el| el.into_builder())
                        }),
                )
            })
        } else {
            self
        }
    }

    /// When the width of the row exceeds the width of its parent, wrap the row's children to the
    /// next line, recursively.
    pub fn multiline(self) -> Self {
        self.with_builder(|builder| {
            builder.with_component::<Node>(|mut node| {
                node.flex_wrap = FlexWrap::Wrap;
                node.flex_basis = Val::Px(0.);
                node.flex_grow = 1.;
            })
        })
    }
}

impl<NodeType: Bundle> Alignable for Row<NodeType> {
    fn layout_direction() -> LayoutDirection {
        LayoutDirection::Row
    }
}
