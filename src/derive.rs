use bevy_camera::visibility::*;
use bevy_ecs::prelude::*;
use bevy_text::prelude::*;
use bevy_transform::prelude::*;
use bevy_ui::{prelude::*, widget::*, *};
use jonmo::signal::Signal;
use paste::paste;

use super::{column::Column, el::El, element::BuilderWrapper, grid::Grid, row::Row, stack::Stack};

// TODO: add link to usage in example challenge 4
/// Implement [haalka](crate)-esque methods for any [`BuilderWrapper`] over the named components,
/// enabling one to quickly add high level signals-powered reactivity to any [`Bundle`], not just [bevy_ui nodes](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ui/src/node_bundles.rs).
///
/// # Example
/// ```
/// use bevy::prelude::*;
/// use haalka::{prelude::*, impl_haalka_methods};
///
/// #[derive(Component, Default)]
/// struct MyComponentA(usize);
///
/// #[derive(Component, Default)]
/// struct MyComponentB {
///     data: usize,
/// }
///
/// #[derive(Bundle, Default)]
/// struct MyBundle {
///     my_component_a: MyComponentA,
///     my_component_b: MyComponentB,
/// }
///
/// #[derive(Default)]
/// struct MyEl(El<MyBundle>);
///
/// impl ElementWrapper for MyEl {
///     type EL = El<MyBundle>;
///     fn element_mut(&mut self) -> &mut Self::EL {
///         &mut self.0
///     }
/// }
///
/// impl_haalka_methods! {
///     MyEl {
///        my_component_a: MyComponentA,
///        some_other_component_idk: MyComponentB,
///     }
/// }
///
/// MyEl::default()
/// .my_component_a(MyComponentA(1))
/// .with_some_other_component_idk(|mut some_other_component_idk| some_other_component_idk.data = 2)
/// .my_component_a_signal(always(3).map(MyComponentA))
/// .on_signal_with_some_other_component_idk(always(4), |mut some_other_component_idk, data| some_other_component_idk.data = data);
/// ```
///
/// # Skipping Signal Derivation
/// Use `#[skip_signal]` before a field to skip generating the `_signal` method for that field.
/// This is useful when the component type doesn't implement [`Clone`] or when you don't need
/// the signal method.
///
/// ```
/// # use bevy::prelude::*;
/// # use haalka::{prelude::*, impl_haalka_methods};
/// # #[derive(Component, Default)]
/// # struct MyComponentA(usize);
/// # #[derive(Component, Default)]
/// # struct MyComponentB { data: usize }
/// # #[derive(Bundle, Default)]
/// # struct MyBundle { my_component_a: MyComponentA, my_component_b: MyComponentB }
/// # #[derive(Default)]
/// # struct MyEl(El<MyBundle>);
/// # impl ElementWrapper for MyEl {
/// #     type EL = El<MyBundle>;
/// #     fn element_mut(&mut self) -> &mut Self::EL { &mut self.0 }
/// # }
/// impl_haalka_methods! {
///     MyEl {
///        my_component_a: MyComponentA,
///        #[skip_signal]
///        some_other_component_idk: MyComponentB,
///     }
/// }
/// // `my_component_a_signal` is available, but `some_other_component_idk_signal` is not
/// ```
#[macro_export]
macro_rules! impl_haalka_methods {
    // Internal rule for common methods shared by all fields
    (@common $field:ident: $field_type:ty) => {
        paste! {
            #[doc = concat!("Set this element's [`", stringify!($field_type), "`] [`Component`].")]
            pub fn $field(mut self, [<$field _option>]: impl Into<Option<$field_type>>) -> Self {
                if let Some($field) = [<$field _option>].into() {
                    self = self.with_builder(|builder| builder.insert($field));
                }
                self
            }

            #[doc = concat!("Run a function with mutable access (via [`Mut`]) to this element's [`", stringify!($field_type), "`] [`Component`] if it exists.")]
            pub fn [<with_ $field>](self, f: impl FnOnce(Mut<$field_type>) + Send + Sync + 'static) -> Self {
                self.with_builder(|builder| builder.with_component::<$field_type>(f))
            }

            #[doc = concat!("Reactively run a function with mutable access (via [`Mut`]) to this element's [`", stringify!($field_type), "`] [`Component`] and the output of the [`Signal`].")]
            pub fn [<on_signal_with_ $field>]<T: Clone + Send + Sync + 'static>(
                self,
                signal: impl Signal<Item = T> + Send + 'static,
                f: impl FnMut(Mut<$field_type>, T) + Send + Sync + 'static,
            ) -> Self {
                self.with_builder(|builder| {
                    builder.on_signal_with_component::<$field_type, T, _, _>(signal, f)
                })
            }
        }
    };
    // Internal rule for fields WITH #[skip_signal] - only common methods
    (@field $el_type:ty, #[skip_signal] $field:ident: $field_type:ty) => {
        $crate::impl_haalka_methods!(@common $field: $field_type);
    };
    // Internal rule for fields WITHOUT #[skip_signal] - common methods + signal method
    (@field $el_type:ty, $field:ident: $field_type:ty) => {
        $crate::impl_haalka_methods!(@common $field: $field_type);
        paste! {
            #[doc = concat!("Reactively set this element's [`", stringify!($field_type), "`] [`Component`]. If the [`Signal`] outputs [`None`], the `C` [`Component`] is removed.")]
            pub fn [<$field _signal>]<S: Signal<Item = Option<$field_type>> + Send + 'static>(self, [<$field _signal>]: S) -> Self {
                self.with_builder(|builder| builder.component_signal([<$field _signal>]))
            }
        }
    };
    // Main entry point
    ($el_type:ty {$($(#[$attr:ident])? $field:ident: $field_type:ty),* $(,)?}) => {
        impl $el_type {
            $(
                $crate::impl_haalka_methods!(@field $el_type, $(#[$attr])? $field: $field_type);
            )*
        }
    };
}

macro_rules! impl_haalka_methods_for_aligners_and_node_bundles {
    ($($el_type:ty),* $(,)?) => {
        $(
            paste! {
                impl_haalka_methods! {
                    $el_type<Node> {
                        node: Node,
                        computed_node: ComputedNode,
                        background_color: BackgroundColor,
                        border_color: BorderColor,
                        border_radius: BorderRadius,
                        box_shadow: BoxShadow,
                        focus_policy: FocusPolicy,
                        scroll_position: ScrollPosition,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                        global_z_index: GlobalZIndex,
                    }
                }
                impl_haalka_methods! {
                    $el_type<ImageNode> {
                        image_node: ImageNode,
                        image_node_size: ImageNodeSize,
                        #[skip_signal]
                        content_size: ContentSize,
                        node: Node,
                        computed_node: ComputedNode,
                        background_color: BackgroundColor,
                        border_color: BorderColor,
                        border_radius: BorderRadius,
                        box_shadow: BoxShadow,
                        focus_policy: FocusPolicy,
                        scroll_position: ScrollPosition,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                        global_z_index: GlobalZIndex,
                    }
                }
                impl_haalka_methods! {
                    $el_type<Text> {
                        text: Text,
                        text_layout: TextLayout,
                        text_font: TextFont,
                        text_color: TextColor,
                        text_node_flags: TextNodeFlags,
                        #[skip_signal]
                        content_size: ContentSize,
                        node: Node,
                        computed_node: ComputedNode,
                        background_color: BackgroundColor,
                        border_color: BorderColor,
                        border_radius: BorderRadius,
                        box_shadow: BoxShadow,
                        focus_policy: FocusPolicy,
                        scroll_position: ScrollPosition,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                        global_z_index: GlobalZIndex,
                    }
                }
                impl_haalka_methods! {
                    $el_type<Button> {
                        interaction: Interaction,
                        node: Node,
                        computed_node: ComputedNode,
                        background_color: BackgroundColor,
                        border_color: BorderColor,
                        border_radius: BorderRadius,
                        box_shadow: BoxShadow,
                        focus_policy: FocusPolicy,
                        scroll_position: ScrollPosition,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                        global_z_index: GlobalZIndex,
                    }
                }
            }
        )*
    }
}

// TODO: how expensive is it to have all these methods ?
impl_haalka_methods_for_aligners_and_node_bundles! {
    El,
    Column,
    Row,
    Stack,
    Grid,
}

// TODO: macro doesn't play nice with generics and chatgpt can't figure it out
// TODO: migrate to aalo's impl_syncers! strategy for this
// MaterialNodeBundle<M: UiMaterial> {
//     node: Node,
//     style: Style,
//     focus_policy: FocusPolicy,
//     transform: Transform,
//     global_transform: GlobalTransform,
//     visibility: Visibility,
//     inherited_visibility: InheritedVisibility,
//     view_visibility: ViewVisibility,
//     z_index: ZIndex,
// },
