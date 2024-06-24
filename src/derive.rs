use bevy::{
    prelude::*,
    text::TextLayoutInfo,
    ui::{
        widget::{TextFlags, UiImageSize},
        ContentSize, FocusPolicy,
    },
};
use futures_signals::signal::Signal;
use paste::paste;

use super::{column::Column, el::El, grid::Grid, raw::RawElWrapper, row::Row, stack::Stack};

// TODO: add link to usage in example challenge 4
/// Implement [haalka](crate)-esque methods for any [`RawElWrapper`] over the named components,
/// enabling one to quickly add high level signals-powered reactivity to any [`Bundle`], not just [bevy_ui nodes](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ui/src/node_bundles.rs).
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use haalka::prelude::*;
///
/// #[derive(Component)]
/// struct MyComponentA(usize);
///
/// #[derive(Component)]
/// struct MyComponentB {
///     data: usize,
/// }
///
/// #[derive(Bundle)]
/// struct MyBundle {
///     my_component_a: MyComponentA,
///     my_component_b: MyComponentB,
/// }
///
/// #[derive(Default)]
/// struct MyEl(RawHaalkaEl);
///
/// impl RawElWrapper for MyEl {
///     fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
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
#[macro_export]
macro_rules! impl_haalka_methods {
    ($el_type:ty {$($field:ident: $field_type:ty),* $(,)?}) => {
        impl $el_type {
            $(
                paste! {
                    #[doc = concat!("Set this element's [`", stringify!($field_type), "`] [`Component`].")]
                    pub fn $field(mut self, [<$field _option>]: impl Into<Option<$field_type>>) -> Self {
                        if let Some($field) = [<$field _option>].into() {
                            self = self.update_raw_el(|raw_el| raw_el.insert($field));
                        }
                        self
                    }

                    #[doc = concat!("Run a function with mutable access (via [`Mut`]) to this element's [`", stringify!($field_type), "`] [`Component`] if it exists.")]
                    pub fn [<with_ $field>](self, f: impl FnOnce(Mut<$field_type>) + Send + 'static) -> Self {
                        self.update_raw_el(|raw_el| raw_el.with_component::<$field_type>(f))
                    }

                    #[doc = concat!("Reactively set this element's [`", stringify!($field_type), "`] [`Component`]. If the [`Signal`] outputs [`None`], the `C` [`Component`] is removed.")]
                    pub fn [<$field _signal>]<S: Signal<Item = $field_type> + Send + 'static>(self, [<$field _signal>]: impl Into<Option<S>>) -> Self {
                        self.update_raw_el(|raw_el| raw_el.component_signal([<$field _signal>]))
                    }

                    #[doc = concat!("Reactively run a function with mutable access (via [`Mut`]) to this element's [`", stringify!($field_type), "`] [`Component`] and the output of the [`Signal`].")]
                    pub fn [<on_signal_with_ $field>]<T: Send + 'static>(
                        self,
                        signal: impl Signal<Item = T> + Send + 'static,
                        f: impl FnMut(Mut<$field_type>, T) + Send + 'static,
                    ) -> Self {
                        self.update_raw_el(|raw_el| {
                            raw_el.on_signal_with_component::<T, $field_type>(signal, f)
                        })
                    }

                }
            )*
        }
    }
}

macro_rules! impl_haalka_methods_for_aligners_and_node_bundles {
    ($($el_type:ty),* $(,)?) => {
        $(
            paste! {
                impl_haalka_methods! {
                    $el_type<NodeBundle> {
                        node: Node,
                        style: Style,
                        background_color: BackgroundColor,
                        border_color: BorderColor,
                        focus_policy: FocusPolicy,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                    }
                }
                impl_haalka_methods! {
                    $el_type<ImageBundle> {
                        node: Node,
                        style: Style,
                        calculated_size: ContentSize,
                        background_color: BackgroundColor,
                        image: UiImage,
                        image_size: UiImageSize,
                        focus_policy: FocusPolicy,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                    }
                }
                impl_haalka_methods! {
                    $el_type<AtlasImageBundle> {
                        node: Node,
                        style: Style,
                        calculated_size: ContentSize,
                        background_color: BackgroundColor,
                        image: UiImage,
                        texture_atlas: TextureAtlas,
                        focus_policy: FocusPolicy,
                        image_size: UiImageSize,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                    }
                }
                impl_haalka_methods! {
                    $el_type<TextBundle> {
                        node: Node,
                        style: Style,
                        text: Text,
                        text_layout_info: TextLayoutInfo,
                        text_flags: TextFlags,
                        calculated_size: ContentSize,
                        focus_policy: FocusPolicy,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
                        background_color: BackgroundColor,
                    }
                }
                impl_haalka_methods! {
                    $el_type<ButtonBundle> {
                        node: Node,
                        button: Button,
                        style: Style,
                        interaction: Interaction,
                        focus_policy: FocusPolicy,
                        background_color: BackgroundColor,
                        border_color: BorderColor,
                        image: UiImage,
                        transform: Transform,
                        global_transform: GlobalTransform,
                        visibility: Visibility,
                        inherited_visibility: InheritedVisibility,
                        view_visibility: ViewVisibility,
                        z_index: ZIndex,
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
