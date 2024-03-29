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

use crate::{Column, El, Grid, RawElWrapper, Row, Stack};

#[macro_export]
macro_rules! impl_haalka_methods {
    ($($el_type:ty => { $($node_type:ty => {$($field:ident: $field_type:ty),* $(,)?}),+ $(,)? }),+ $(,)?) => {
        $(
            $(
                paste! {
                    impl $el_type<$node_type> {
                        $(
                            paste! {
                                pub fn $field(self, $field: $field_type) -> Self {
                                    self.update_raw_el(|raw_el| raw_el.insert($field))
                                }

                                pub fn [<with_ $field>](self, f: impl FnOnce(&mut $field_type) + Send + 'static) -> Self {
                                    self.update_raw_el(|raw_el| raw_el.with_component::<$field_type>(f))
                                }

                                pub fn [<$field _signal>](self, [<$field _signal>]: impl Signal<Item = $field_type> + Send + 'static) -> Self {
                                    self.update_raw_el(|raw_el| raw_el.component_signal([<$field _signal>]))
                                }

                                pub fn [<on_signal_with_ $field>]<T: Send + 'static>(
                                    self,
                                    signal: impl Signal<Item = T> + Send + 'static,
                                    f: impl FnMut(&mut $field_type, T) + Send + 'static,
                                ) -> Self {
                                    self.update_raw_el(|raw_el| {
                                        raw_el.on_signal_with_component::<T, $field_type>(signal, f)
                                    })
                                }
                            }
                        )*
                    }
                }
            )*
        )*
    };
}

impl_haalka_methods! {
    El => {
        NodeBundle => {
            node: bevy::ui::Node,
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
        },
        ImageBundle => {
            node: bevy::ui::Node,
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
        },
        AtlasImageBundle => {
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        },
        TextBundle => {
            node: bevy::ui::Node,
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
        },
        ButtonBundle => {
            node: bevy::ui::Node,
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
        },
    },
    Column => {
        NodeBundle => {
            node: bevy::ui::Node,
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
        },
        ImageBundle => {
            node: bevy::ui::Node,
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
        },
        AtlasImageBundle => {
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        },
        TextBundle => {
            node: bevy::ui::Node,
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
        },
        ButtonBundle => {
            node: bevy::ui::Node,
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
        },
    },
    Row => {
        NodeBundle => {
            node: bevy::ui::Node,
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
        },
        ImageBundle => {
            node: bevy::ui::Node,
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
        },
        AtlasImageBundle => {
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        },
        TextBundle => {
            node: bevy::ui::Node,
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
        },
        ButtonBundle => {
            node: bevy::ui::Node,
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
        },
    },
    Stack => {
        NodeBundle => {
            node: bevy::ui::Node,
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
        },
        ImageBundle => {
            node: bevy::ui::Node,
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
        },
        AtlasImageBundle => {
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        },
        TextBundle => {
            node: bevy::ui::Node,
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
        },
        ButtonBundle => {
            node: bevy::ui::Node,
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
        },
    },
    Grid => {
        NodeBundle => {
            node: bevy::ui::Node,
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
        },
        ImageBundle => {
            node: bevy::ui::Node,
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
        },
        AtlasImageBundle => {
            node: bevy::ui::Node,
            style: Style,
            calculated_size: ContentSize,
            background_color: BackgroundColor,
            texture_atlas: Handle<TextureAtlas>,
            texture_atlas_image: UiTextureAtlasImage,
            focus_policy: FocusPolicy,
            image_size: UiImageSize,
            transform: Transform,
            global_transform: GlobalTransform,
            visibility: Visibility,
            inherited_visibility: InheritedVisibility,
            view_visibility: ViewVisibility,
            z_index: ZIndex,
        },
        TextBundle => {
            node: bevy::ui::Node,
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
        },
        ButtonBundle => {
            node: bevy::ui::Node,
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
        },
    },
    // TODO: macros don't play nice with generics
    // MaterialNodeBundle<M: UiMaterial> => {
    //     node: bevy::ui::Node,
    //     style: Style,
    //     focus_policy: FocusPolicy,
    //     transform: Transform,
    //     global_transform: GlobalTransform,
    //     visibility: Visibility,
    //     inherited_visibility: InheritedVisibility,
    //     view_visibility: ViewVisibility,
    //     z_index: ZIndex,
    // },
}
