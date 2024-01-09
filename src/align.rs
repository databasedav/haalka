use std::collections::BTreeSet;

use bevy::prelude::*;
use futures_signals::signal::{Signal, SignalExt, BoxSignal};

use crate::{RawElWrapper, RawElement, IntoOptionElement};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    alignments: BTreeSet<Alignment>,
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

fn register_align_signal<REW: RawElWrapper>(element: REW, align_signal: impl Signal<Item = Option<Vec<Alignment>>> + Send + 'static, apply_alignment: fn(&mut Style, Alignment, AddRemove)) -> REW {
    let mut last_alignments_option: Option<Vec<Alignment>> = None;
    element.update_raw_el(|raw_el| raw_el.on_signal_with_component::<Style, Option<Vec<Alignment>>>(align_signal, move |style, aligns_option| {
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
            last_alignments_option = if !alignments.is_empty() { Some(alignments) } else { None };
        } else {
            if let Some(last_aligns) = last_alignments_option.take() {
                for align in last_aligns {
                    apply_alignment(style, align, AddRemove::Remove)
                }
            }
        }
    }))
}

pub trait Alignable: RawElWrapper {
    fn align_mut(&mut self) -> &mut Option<AlignHolder>;

    fn align(mut self, align: Align) -> Self {
        *self.align_mut() = Some(AlignHolder::Align(align));
        self
    }

    fn align_signal(mut self, align_option_signal: impl Signal<Item = Option<Align>> + Send + 'static) -> Self {
        *self.align_mut() = Some(AlignHolder::AlignSignal(align_option_signal.boxed()));
        self
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove);

    fn align_content(self, align: Align) -> Self {
        self.update_raw_el(|raw_el| {
            raw_el.with_component::<Style>(|style| {
                for alignment in align.alignments {
                    Self::apply_content_alignment(style, alignment, AddRemove::Add)
                }
            })
        })
    }

    fn align_content_signal(self, align_option_signal: impl Signal<Item = Option<Align>> + Send + 'static) -> Self {
        register_align_signal(self, align_option_signal.map(|align_option| align_option.map(|align| align.alignments.into_iter().collect())), Self::apply_content_alignment)
    }
}

pub trait ChildAlignable: Alignable where Self: 'static {
    fn update_style(_style: &mut Style) {}  // only Stack requires base updates

    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove);

    fn manage<NodeType: Bundle, Child: RawElWrapper + Alignable>(mut child: Child) -> Child {
        child = child.update_raw_el(|raw_el| raw_el.with_component::<Style>(Self::update_style));
        // TODO: this .take means that child can't be passed around parents without losing align info, but this can be easily added if desired
        if let Some(align) = child.align_mut().take() {
            match align {
                AlignHolder::Align(align) => {
                    child = child.update_raw_el(|raw_el| raw_el.with_component::<Style>(move |style| {
                        for align in align.alignments {
                            Self::apply_alignment(style, align, AddRemove::Add)
                        }
                    }))
                }
                AlignHolder::AlignSignal(align_option_signal) => {
                    child = register_align_signal(
                        child,
                        {
                            align_option_signal.map(|align_option|
                                align_option.map(|align| align.alignments.into_iter().collect())
                            )
                        },
                        Self::apply_alignment
                    )
                }
            }
        }
        child
    }
}

// TODO: ideally want to be able to process raw el's as well if they need some, but this is convenient for now ...
pub trait ChildProcessable: Alignable {
    fn process_child<IOE: IntoOptionElement>(child_option: IOE) -> std::option::Option<<IOE as IntoOptionElement>::EL>;
}

impl<CA: ChildAlignable> ChildProcessable for CA {
    fn process_child<IOE: IntoOptionElement>(child_option: IOE) -> std::option::Option<<IOE as IntoOptionElement>::EL>
    {
        child_option.into_option_element().map(|mut child| {
            child = <Self as ChildAlignable>::manage::<<<IOE as IntoOptionElement>::EL as RawElement>::NodeType, <IOE as IntoOptionElement>::EL>(child);
            child
        })
    }
}
