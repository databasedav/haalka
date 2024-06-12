use std::{collections::BTreeSet, ops::Not};

use bevy::prelude::*;
use futures_signals::signal::{BoxSignal, Signal, SignalExt};

use crate::{Column, El, ElementWrapper, Grid, RawElWrapper, RawHaalkaEl, Row, Stack};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Alignment {
    Top,
    Bottom,
    Left,
    Right,
    CenterX,
    CenterY,
}

#[derive(Clone, Default)]
pub struct Align {
    pub alignments: BTreeSet<Alignment>,
}

impl Align {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn center() -> Self {
        Self::default().center_x().center_y()
    }

    pub fn center_x(mut self) -> Self {
        self.alignments.insert(Alignment::CenterX);
        self.alignments.remove(&Alignment::Left);
        self.alignments.remove(&Alignment::Right);
        self
    }

    pub fn center_y(mut self) -> Self {
        self.alignments.insert(Alignment::CenterY);
        self.alignments.remove(&Alignment::Top);
        self.alignments.remove(&Alignment::Bottom);
        self
    }

    pub fn top(mut self) -> Self {
        self.alignments.insert(Alignment::Top);
        self.alignments.remove(&Alignment::CenterY);
        self.alignments.remove(&Alignment::Bottom);
        self
    }

    pub fn bottom(mut self) -> Self {
        self.alignments.insert(Alignment::Bottom);
        self.alignments.remove(&Alignment::CenterY);
        self.alignments.remove(&Alignment::Top);
        self
    }

    pub fn left(mut self) -> Self {
        self.alignments.insert(Alignment::Left);
        self.alignments.remove(&Alignment::CenterX);
        self.alignments.remove(&Alignment::Right);
        self
    }

    pub fn right(mut self) -> Self {
        self.alignments.insert(Alignment::Right);
        self.alignments.remove(&Alignment::CenterX);
        self.alignments.remove(&Alignment::Left);
        self
    }
}

pub enum AlignHolder {
    Align(Align),
    AlignSignal(BoxSignal<'static, Option<Align>>),
}

pub enum AddRemove {
    Add,
    Remove,
}

pub(crate) fn register_align_signal<REW: RawElWrapper>(
    element: REW,
    align_signal: impl Signal<Item = Option<Vec<Alignment>>> + Send + 'static,
    apply_alignment: fn(&mut Style, Alignment, AddRemove),
) -> REW {
    let mut last_alignments_option: Option<Vec<Alignment>> = None;
    element.update_raw_el(|raw_el| {
        raw_el.on_signal_with_component::<Option<Vec<Alignment>>, Style>(align_signal, move |style, aligns_option| {
            if let Some(alignments) = aligns_option {
                if let Some(mut last_alignments) = last_alignments_option.take() {
                    last_alignments.retain(|align| !alignments.contains(align));
                    for alignment in last_alignments {
                        apply_alignment(style, alignment, AddRemove::Remove)
                    }
                }
                for alignment in &alignments {
                    apply_alignment(style, *alignment, AddRemove::Add)
                }
                last_alignments_option = alignments.is_empty().not().then_some(alignments);
            } else {
                if let Some(last_aligns) = last_alignments_option.take() {
                    for align in last_aligns {
                        apply_alignment(style, align, AddRemove::Remove)
                    }
                }
            }
        })
    })
}

pub trait Alignable: RawElWrapper {
    fn alignable_type(&mut self) -> Option<AlignableType> {
        None
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder>;

    fn align(mut self, align_option: impl Into<Option<Align>>) -> Self
    where
        Self: Sized,
    {
        if let Some(align) = align_option.into() {
            *self.align_mut() = Some(AlignHolder::Align(align));
        }
        self
    }

    fn align_signal<S: Signal<Item = Option<Align>> + Send + 'static>(
        mut self,
        align_option_signal_option: impl Into<Option<S>>,
    ) -> Self
    where
        Self: Sized,
    {
        if let Some(align_option_signal) = align_option_signal_option.into() {
            *self.align_mut() = Some(AlignHolder::AlignSignal(align_option_signal.boxed()));
        }
        self
    }

    // exists for the `AlignabilityFacade`
    fn apply_content_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        Self::apply_content_alignment
    }

    fn apply_content_alignment(_style: &mut Style, _alignment: Alignment, _action: AddRemove);

    fn align_content(mut self, align_option: impl Into<Option<Align>>) -> Self {
        if let Some(align) = align_option.into() {
            let apply_content_alignment = self.apply_content_alignment_wrapper();
            self = self.update_raw_el(move |raw_el| {
                raw_el.with_component::<Style>(move |style| {
                    for alignment in align.alignments {
                        apply_content_alignment(style, alignment, AddRemove::Add);
                    }
                })
            });
        }
        self
    }

    fn align_content_signal<S: Signal<Item = Option<Align>> + Send + 'static>(
        mut self,
        align_option_signal_option: impl Into<Option<S>>,
    ) -> Self {
        if let Some(align_option_signal) = align_option_signal_option.into() {
            let apply_content_alignment = self.apply_content_alignment_wrapper();
            self = register_align_signal(
                self,
                align_option_signal
                    .map(|align_option| align_option.map(|align| align.alignments.into_iter().collect())),
                apply_content_alignment,
            );
        }
        self
    }
}

pub trait ChildAlignable
where
    Self: 'static,
{
    fn update_style(_style: &mut Style) {} // only some require base updates

    // exists for the `AlignabilityFacade`
    fn apply_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        Self::apply_alignment
    }

    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove);

    fn align_child<Child: RawElWrapper + Alignable>(
        mut child: Child,
        apply_alignment: fn(&mut Style, Alignment, AddRemove),
    ) -> Child {
        child = child.update_raw_el(|raw_el| raw_el.with_component::<Style>(Self::update_style));
        // TODO: this .take means that child can't be passed around parents without losing align
        // info, but this can be easily added if desired
        if let Some(align) = child.align_mut().take() {
            match align {
                AlignHolder::Align(align) => {
                    child = child.update_raw_el(|raw_el| {
                        raw_el.with_component::<Style>(move |style| {
                            for align in align.alignments {
                                apply_alignment(style, align, AddRemove::Add)
                            }
                        })
                    })
                }
                AlignHolder::AlignSignal(align_option_signal) => {
                    child = register_align_signal(
                        child,
                        {
                            align_option_signal
                                .map(|align_option| align_option.map(|align| align.alignments.into_iter().collect()))
                        },
                        apply_alignment,
                    )
                }
            }
        }
        child
    }
}

impl<EW: ElementWrapper> Alignable for EW {
    fn alignable_type(&mut self) -> Option<AlignableType> {
        self.element_mut().alignable_type()
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        self.element_mut().align_mut()
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        EW::EL::apply_content_alignment(style, alignment, action);
    }
}

impl<EW: ElementWrapper + 'static> ChildAlignable for EW {
    fn update_style(style: &mut Style) {
        EW::EL::update_style(style);
    }

    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove) {
        EW::EL::apply_alignment(style, align, action);
    }
}

#[derive(Clone, Copy)]
pub enum AlignableType {
    El,
    Column,
    Row,
    Stack,
    Grid,
    // TODO: allow specifying custom alignment functions
}

pub struct AlignabilityFacade {
    raw_el: RawHaalkaEl,
    align: Option<AlignHolder>,
    alignable_type: AlignableType,
}

impl AlignabilityFacade {
    pub(crate) fn new(raw_el: RawHaalkaEl, align: Option<AlignHolder>, alignable_type: AlignableType) -> Self {
        Self {
            raw_el,
            align,
            alignable_type,
        }
    }
}

impl RawElWrapper for AlignabilityFacade {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        self.raw_el.raw_el_mut()
    }
}

impl Alignable for AlignabilityFacade {
    fn alignable_type(&mut self) -> Option<AlignableType> {
        Some(self.alignable_type)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        match self.alignable_type {
            AlignableType::El => El::<NodeBundle>::apply_content_alignment,
            AlignableType::Column => Column::<NodeBundle>::apply_content_alignment,
            AlignableType::Row => Row::<NodeBundle>::apply_content_alignment,
            AlignableType::Stack => Stack::<NodeBundle>::apply_content_alignment,
            AlignableType::Grid => Grid::<NodeBundle>::apply_content_alignment,
        }
    }

    fn apply_content_alignment(_style: &mut Style, _alignment: Alignment, _action: AddRemove) {}
}

impl ChildAlignable for AlignabilityFacade {
    fn apply_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        match self.alignable_type {
            AlignableType::El => El::<NodeBundle>::apply_alignment,
            AlignableType::Column => Column::<NodeBundle>::apply_alignment,
            AlignableType::Row => Row::<NodeBundle>::apply_alignment,
            AlignableType::Stack => Stack::<NodeBundle>::apply_alignment,
            AlignableType::Grid => Grid::<NodeBundle>::apply_alignment,
        }
    }

    fn apply_alignment(_style: &mut Style, _align: Alignment, _action: AddRemove) {}
}
