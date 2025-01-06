//! Simple alignment semantics ported from [MoonZoon](https://github.com/MoonZoon/MoonZoon)'s [`align`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/style/align.rs) and [`align_content`](https://github.com/MoonZoon/MoonZoon/blob/main/crates/zoon/src/style/align_content.rs).
//!
//! An [`Element`](`super::element::Element`) can be aligned in nine different areas in relation to
//! its parent: top left, top center, top right, center left, center, center right, bottom left,
//! bottom center, and bottom right. This provides a simple and clear to way to declare alignment as
//! a thin layer on top of bevy_ui's flexbox implementation.
//!
//! [`Align`]s can be specified on individual elements using [`.align`](`Alignable::align`) and
//! [`.align_signal`](`Alignable::align_signal`) or to all children using
//! [`.align_content`](`Alignable::align_content`) and
//! [`.align_content_signal`](`Alignable::align_content_signal`). See the [align](https://github.com/databasedav/haalka/blob/main/examples/align.rs)
//! example for how each [`Align`] behaves for each built-in alignable type: [`El`], [`Column`],
//! [`Row`], [`Stack`], and [`Grid`].
//!
//! # Notes
//! [`Stack`] and [`Grid`] children (read: children that are either a [`Stack`] or a [`Grid`], not
//! the children *of* [`Stack`]s or [`Grid`]s) do not behave as expected when aligned with a
//! parent's [`.align_content`](`Alignable::align_content`) or
//! [`.align_content_signal`](`Alignable::align_content_signal`); this is a known issue and one can
//! simply align the [`Stack`] or [`Grid`] themselves as workaround.

use std::{collections::BTreeSet, ops::Not};

use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use futures_signals::signal::{BoxSignal, Signal, SignalExt};

use super::{
    column::Column,
    el::El,
    element::ElementWrapper,
    grid::Grid,
    raw::{DeferredUpdaterAppendDirection, RawElWrapper, RawHaalkaEl},
    row::Row,
    stack::Stack,
};

// TODO: replace moonzoon github links with docs.rs links once moonzoon crate published
// TODO: create and link issue for Stack and Grid content alignment behavior

/// Holder of composable [`Alignment`]s.
#[derive(Clone, Default)]
pub struct Align {
    alignments: BTreeSet<Alignment>,
}

#[allow(missing_docs)]
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

/// Composable alignment variants. See [`Align`].
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Alignment {
    Top,
    Bottom,
    Left,
    Right,
    CenterX,
    CenterY,
}

/// Holder for [`Align`] data. See [`Alignable`] and [`ChildAlignable`].
pub enum AlignHolder {
    /// Static
    Align(Align),
    /// Reactive
    AlignSignal(BoxSignal<'static, Option<Align>>),
}

/// Whether to add or remove an [`Alignment`]. See [`Alignable`] and [`ChildAlignable`].
#[allow(missing_docs)]
pub enum AddRemove {
    Add,
    Remove,
}

fn register_align_signal<REW: RawElWrapper>(
    element: REW,
    align_signal: impl Signal<Item = Option<Vec<Alignment>>> + Send + 'static,
    apply_alignment: fn(&mut Style, Alignment, AddRemove),
) -> REW {
    let mut last_alignments_option: Option<Vec<Alignment>> = None;
    element.update_raw_el(|raw_el| {
        raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
            raw_el.on_signal_with_component::<Option<Vec<Alignment>>, Style>(
                align_signal,
                move |mut style, aligns_option| {
                    if let Some(alignments) = aligns_option {
                        // TODO: confirm that this last alignment removal strategy is working as intended
                        if let Some(mut last_alignments) = last_alignments_option.take() {
                            last_alignments.retain(|align| !alignments.contains(align));
                            for alignment in last_alignments {
                                apply_alignment(&mut style, alignment, AddRemove::Remove)
                            }
                        }
                        for alignment in &alignments {
                            apply_alignment(&mut style, *alignment, AddRemove::Add)
                        }
                        last_alignments_option = alignments.is_empty().not().then_some(alignments);
                    } else if let Some(last_aligns) = last_alignments_option.take() {
                        for align in last_aligns {
                            apply_alignment(&mut style, align, AddRemove::Remove)
                        }
                    }
                },
            )
        })
    })
}

/// [`Alignable`] types can align themselves (although application of self alignment is managed by
/// [`ChildAlignable`]) and their children.
pub trait Alignable: RawElWrapper {
    /// The [`Aligner`] of this type. Used for indirection in [`AlignabilityFacade`].
    fn aligner(&mut self) -> Option<Aligner> {
        None
    }

    /// Mutable reference to the [`Align`] data of this type.
    fn align_mut(&mut self) -> &mut Option<AlignHolder>;

    /// Statically align this element, itself. See [`Align`].
    fn align(mut self, align_option: impl Into<Option<Align>>) -> Self
    where
        Self: Sized,
    {
        if let Some(align) = align_option.into() {
            *self.align_mut() = Some(AlignHolder::Align(align));
        }
        self
    }

    /// Reactively align this element, itself. See [`Align`].
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

    /// Allows implementor to override the content alignment processing function. The `&self` can
    /// be used to alter the alignment strategy based on data on the type itself. See
    /// [`AlignabilityFacade::apply_alignment_wrapper`] for an example.
    fn apply_content_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        Self::apply_content_alignment
    }

    /// How to modify the style of this element given a content alignment and whether to add or
    /// remove it.
    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove);

    /// Statically align the children of this element. See [`Align`].
    ///
    /// # Notes
    /// [`Stack`] and [`Grid`] children (read: children that are either a [`Stack`] or a [`Grid`],
    /// not the children *of* [`Stack`]s or [`Grid`]s) do not behave as expected when aligned
    /// with a parent's [`.align_content`](`Alignable::align_content`) or
    /// [`.align_content_signal`](`Alignable::align_content_signal`); this is a known issue and one
    /// can simply align the [`Stack`] or [`Grid`] themselves as workaround.
    fn align_content(mut self, align_option: impl Into<Option<Align>>) -> Self {
        if let Some(align) = align_option.into() {
            let apply_content_alignment = self.apply_content_alignment_wrapper();
            self = self.update_raw_el(move |raw_el| {
                raw_el.with_component::<Style>(move |mut style| {
                    for alignment in align.alignments {
                        apply_content_alignment(&mut style, alignment, AddRemove::Add);
                    }
                })
            });
        }
        self
    }

    /// Reactively align the children of this element. See [`Align`].
    ///
    /// # Notes
    /// [`Stack`] and [`Grid`] children (read: children that are either a [`Stack`] or a [`Grid`],
    /// not the children *of* [`Stack`]s or [`Grid`]s) do not behave as expected when aligned
    /// with a parent's [`.align_content`](`Alignable::align_content`) or
    /// [`.align_content_signal`](`Alignable::align_content_signal`); this is a known issue and one
    /// can simply align the [`Stack`] or [`Grid`] themselves as workaround.
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

/// [`ChildAlignable`] types process and apply the [`Align`] data that their children specify to self align. This is an emulation of the [CSS child combinator](https://developer.mozilla.org/en-US/docs/Web/CSS/Child_combinator).
pub trait ChildAlignable
where
    Self: 'static,
{
    /// Static style modifications for children of this type.
    fn update_style(_style: Mut<Style>) {} // only some require base updates

    /// Allows implementor to override the self alignment processing function. The `&self`
    /// can be used to alter the alignment strategy based on data on the type itself. See
    /// [`AlignabilityFacade::apply_alignment_wrapper`] for an example.
    fn apply_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        Self::apply_alignment
    }

    /// How to modify the style of children of this element given a self alignment and whether to
    /// add or remove it.
    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove);

    /// Align child based on its [`Align`] data and processing defined by the type of its parent.
    fn align_child<Child: RawElWrapper + Alignable>(
        mut child: Child,
        apply_alignment: fn(&mut Style, Alignment, AddRemove),
    ) -> Child {
        child = child.update_raw_el(|raw_el| {
            raw_el.defer_update(DeferredUpdaterAppendDirection::Back, |raw_el| {
                raw_el.with_component::<Style>(Self::update_style)
            })
        });
        // TODO: this .take means that child can't be passed around parents without losing align
        // info, but this can be easily added if desired
        if let Some(align) = child.align_mut().take() {
            match align {
                AlignHolder::Align(align) => {
                    child = child.update_raw_el(|raw_el| {
                        raw_el.defer_update(DeferredUpdaterAppendDirection::Back, move |raw_el| {
                            raw_el.with_component::<Style>(move |mut style| {
                                for align in align.alignments {
                                    apply_alignment(&mut style, align, AddRemove::Add)
                                }
                            })
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
    fn aligner(&mut self) -> Option<Aligner> {
        self.element_mut().aligner()
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        self.element_mut().align_mut()
    }

    fn apply_content_alignment(style: &mut Style, alignment: Alignment, action: AddRemove) {
        EW::EL::apply_content_alignment(style, alignment, action);
    }
}

impl<EW: ElementWrapper + 'static> ChildAlignable for EW {
    fn update_style(style: Mut<Style>) {
        EW::EL::update_style(style);
    }

    fn apply_alignment(style: &mut Style, align: Alignment, action: AddRemove) {
        EW::EL::apply_alignment(style, align, action);
    }
}

/// Exhaustive variants of alignable definitions; used for type indirection in
/// [`AlignabilityFacade`].
#[derive(Clone, Copy)]
pub enum Aligner {
    /// [`El`](`super::el::El`)
    El,
    /// [`Column`](`super::column::Column`)
    Column,
    /// [`Row`](`super::row::Row`)
    Row,
    /// [`Stack`](`super::stack::Stack`)
    Stack,
    /// [`Grid`](`super::grid::Grid`)
    Grid,
    // TODO: allow specifying custom alignment functions
}

/// Provides type indirection for built-in alignable types, enabling simple "type erasure" via
/// [`TypeEraseable::type_erase`](`super::element::TypeEraseable::type_erase`).
pub struct AlignabilityFacade {
    raw_el: RawHaalkaEl,
    align: Option<AlignHolder>,
    aligner: Aligner,
}

impl AlignabilityFacade {
    pub(crate) fn new(raw_el: RawHaalkaEl, align: Option<AlignHolder>, aligner: Aligner) -> Self {
        Self { raw_el, align, aligner }
    }
}

impl RawElWrapper for AlignabilityFacade {
    fn raw_el_mut(&mut self) -> &mut RawHaalkaEl {
        &mut self.raw_el
    }
}

impl Alignable for AlignabilityFacade {
    fn aligner(&mut self) -> Option<Aligner> {
        Some(self.aligner)
    }

    fn align_mut(&mut self) -> &mut Option<AlignHolder> {
        &mut self.align
    }

    fn apply_content_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        match self.aligner {
            Aligner::El => El::<NodeBundle>::apply_content_alignment,
            Aligner::Column => Column::<NodeBundle>::apply_content_alignment,
            Aligner::Row => Row::<NodeBundle>::apply_content_alignment,
            Aligner::Stack => Stack::<NodeBundle>::apply_content_alignment,
            Aligner::Grid => Grid::<NodeBundle>::apply_content_alignment,
        }
    }

    fn apply_content_alignment(_style: &mut Style, _alignment: Alignment, _action: AddRemove) {}
}

impl ChildAlignable for AlignabilityFacade {
    fn apply_alignment_wrapper(&self) -> fn(&mut Style, Alignment, AddRemove) {
        match self.aligner {
            Aligner::El => El::<NodeBundle>::apply_alignment,
            Aligner::Column => Column::<NodeBundle>::apply_alignment,
            Aligner::Row => Row::<NodeBundle>::apply_alignment,
            Aligner::Stack => Stack::<NodeBundle>::apply_alignment,
            Aligner::Grid => Grid::<NodeBundle>::apply_alignment,
        }
    }

    fn apply_alignment(_style: &mut Style, _align: Alignment, _action: AddRemove) {}
}
