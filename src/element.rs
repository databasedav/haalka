use bevy::ecs::bundle::Bundle;

use crate::{ChildAlignable, ChildProcessable, RawElWrapper, RawElement, RawHaalkaEl};

pub trait ElementWrapper {
    type EL: RawElWrapper + ChildAlignable;
    fn element_mut(&mut self) -> &mut Self::EL;
}

impl<EW: ElementWrapper> RawElWrapper for EW {
    type NodeType = <<EW as ElementWrapper>::EL as RawElWrapper>::NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<Self::NodeType> {
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

impl<NodeType: Bundle> RawElWrapper for RawHaalkaEl<NodeType> {
    type NodeType = NodeType;
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl<NodeType> {
        self
    }
}

pub trait Element: RawElement + ChildProcessable {}

impl<T: RawElement + ChildProcessable> Element for T {}
