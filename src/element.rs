use crate::{
    align::{AlignabilityFacade, AlignableType},
    Alignable, ChildAlignable, RawElWrapper, RawElement, RawHaalkaEl,
};
use bevy::prelude::*;

pub trait ElementWrapper: Sized {
    type EL: RawElWrapper + Alignable + ChildAlignable;
    fn element_mut(&mut self) -> &mut Self::EL;
}

impl<EW: ElementWrapper> RawElWrapper for EW {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.element_mut().raw_el_mut()
    }
}

pub trait IntoElement {
    type EL: Element;
    fn into_element(self) -> Self::EL;
}

impl<T: Element> IntoElement for T {
    type EL = T;
    fn into_element(self) -> Self::EL {
        self
    }
}

pub trait IntoOptionElement {
    type EL: Element;
    fn into_option_element(self) -> Option<Self::EL>;
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for Option<IE> {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        self.map(|into_element| into_element.into_element())
    }
}

impl<E: Element, IE: IntoElement<EL = E>> IntoOptionElement for IE {
    type EL = E;
    fn into_option_element(self) -> Option<Self::EL> {
        Some(self.into_element())
    }
}

impl RawElWrapper for RawHaalkaEl {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self
    }
}

pub trait Element: RawElement + Alignable + ChildAlignable {}

impl<T: RawElement + Alignable + ChildAlignable> Element for T {}

pub trait TypeEraseable {
    fn type_erase(self) -> AlignabilityFacade;
}

impl<T: Alignable> TypeEraseable for T {
    fn type_erase(mut self) -> AlignabilityFacade {
        let alignable_type = self.alignable_type().unwrap_or(AlignableType::El);
        let (align_option, raw_el) = (self.align_mut().take(), self.into_raw());
        AlignabilityFacade::new(raw_el, align_option, alignable_type)
    }
}

#[derive(Resource)]
pub struct UiRoot(pub Entity);

pub trait UiRootable {
    fn ui_root(self) -> Self;
}

impl<E: Element> UiRootable for E {
    fn ui_root(self) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.on_spawn(|world, entity| {
                world.insert_resource(UiRoot(entity));
            })
        })
    }
}
