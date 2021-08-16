#![deny(
    missing_docs,
    nonstandard_style,
    unused_parens,
    unused_qualifications,
    rust_2018_idioms,
    rust_2018_compatibility,
    future_incompatible,
    missing_copy_implementations
)]
// These are helpful to enable while we're building the docs.
// #![warn(clippy::missing_docs_in_private_items)]
// #![warn(clippy::missing_panics_doc)]
// #![warn(missing_docs)]

//! This is the main dependency of
//! [Miratope](https://github.com/OfficialURL/miratope-rs). It contains all code
//! to build and name [`Abstract`] and [`Concrete`](conc::Concrete) polytopes
//! alike.
//!
//! If you're interested in actually rendering polytopes, you might want to take
//! a look at the [`miratope`](https://crates.io/crates/miratope) crate instead.

pub mod abs;
pub mod conc;
pub mod geometry;
pub mod group;

use std::{collections::HashSet, error::Error, iter};

use abs::{
    elements::{Ranks, SectionRef},
    flag::{Flag, FlagIter, OrientedFlag, OrientedFlagIter},
    Abstract, ElementMap, Ranked,
};

use vec_like::VecLike;

/// The names for 0-elements, 1-elements, 2-elements, and so on.
const ELEMENT_NAMES: [&str; 12] = [
    "", "Vertices", "Edges", "Faces", "Cells", "Tera", "Peta", "Exa", "Zetta", "Yotta", "Xenna",
    "Daka",
];

/// The word "Components".
const COMPONENTS: &str = "Components";

/// A trait containing the constants associated to each floating point type.
pub trait Float:
    'static
    + nalgebra::Scalar
    + nalgebra::RealField
    + ordered_float::Float
    + Default
    + std::fmt::Display
    + std::str::FromStr
    + serde::Serialize
    + Copy
{
    /// A default epsilon value. Used in general floating point operations that
    /// would return zero given infinite precision.
    // todo: just put it in the methods themselves.
    const EPS: Self;

    /// 0
    const ZERO: Self;

    /// 1
    const ONE: Self;

    /// 2
    const TWO: Self;

    /// 3
    const THREE: Self;

    /// 4
    const FOUR: Self;

    /// Archimedes' constant (π)
    const PI: Self;

    /// The full circle constant (τ)
    ///
    /// Equal to 2π.
    const TAU: Self;

    /// sqrt(2)
    const SQRT_2: Self;

    /// sqrt(2) / 2
    const HALF_SQRT_2: Self;

    /// sqrt(3)
    const SQRT_3: Self;

    /// sqrt(5)
    const SQRT_5: Self;

    /// Takes the square root of a float.
    fn fsqrt(self) -> Self {
        <Self as ordered_float::Float>::sqrt(self)
    }

    /// Takes the absolute value of a float.
    fn fabs(self) -> Self {
        <Self as ordered_float::Float>::abs(self)
    }

    /// Takes the sine of a float.
    fn fsin(self) -> Self {
        <Self as ordered_float::Float>::sin(self)
    }

    /// Takes the cosine of a float.
    fn fcos(self) -> Self {
        <Self as ordered_float::Float>::cos(self)
    }

    /// Takes the sine and cosine of a float.
    fn fsin_cos(self) -> (Self, Self) {
        <Self as ordered_float::Float>::sin_cos(self)
    }

    /// Makes a float from a `f64`.
    fn f64(f: f64) -> Self;

    /// Makes a float from a `usize`.
    fn usize(u: usize) -> Self;

    /// Makes a float from a `u32`.
    fn u32(u: u32) -> Self;
}

/// Constants for `f32`.
impl Float for f32 {
    const EPS: f32 = 1e-5;
    const ZERO: f32 = 0.0;
    const ONE: f32 = 1.0;
    const TWO: f32 = 2.0;
    const THREE: f32 = 3.0;
    const FOUR: f32 = 4.0;
    const PI: f32 = std::f32::consts::PI;
    const TAU: f32 = std::f32::consts::TAU;
    const SQRT_2: f32 = std::f32::consts::SQRT_2;
    const HALF_SQRT_2: f32 = f32::SQRT_2 / 2.0;
    const SQRT_3: f32 = 1.7320508;
    const SQRT_5: f32 = 2.236068;

    fn f64(f: f64) -> Self {
        f as Self
    }

    fn usize(u: usize) -> Self {
        u as Self
    }

    fn u32(u: u32) -> Self {
        u as Self
    }
}

/// Constants for `f64`.
impl Float for f64 {
    const EPS: f64 = 1e-9;
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO: f64 = 2.0;
    const THREE: f64 = 3.0;
    const FOUR: f64 = 4.0;
    const PI: f64 = std::f64::consts::PI;
    const TAU: f64 = std::f64::consts::TAU;
    const SQRT_2: f64 = std::f64::consts::SQRT_2;
    const HALF_SQRT_2: f64 = f64::SQRT_2 / 2.0;
    const SQRT_3: f64 = 1.7320508075688772;
    const SQRT_5: f64 = 2.23606797749979;

    fn f64(f: f64) -> Self {
        f as Self
    }

    fn usize(u: usize) -> Self {
        u as Self
    }

    fn u32(u: u32) -> Self {
        u as Self
    }
}

/// Represents an error in a concrete dual, in which a facet with a given index
/// passes through the inversion center.
#[derive(Clone, Copy, Debug)]
pub struct DualError(usize);

impl std::fmt::Display for DualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "facet {} passes through inversion center", self.0)
    }
}

impl Error for DualError {}

/// Gets the precalculated value for n!.
fn factorial(n: usize) -> u32 {
    /// Precalculated factorials from 0! to 13!.
    const FACTORIALS: [u32; 13] = [
        1, 1, 2, 6, 24, 120, 720, 5040, 40320, 362880, 3628800, 39916800, 479001600,
    ];

    FACTORIALS[n]
}

impl<T: Polytope> Ranked for T {
    fn ranks(&self) -> &Ranks {
        &self.abs().ranks
    }

    fn ranks_mut(&mut self) -> &mut Ranks {
        &mut self.abs_mut().ranks
    }

    fn into_ranks(self) -> Ranks {
        self.into_abs().ranks
    }
}

/// The trait for methods common to all polytopes.
pub trait Polytope: Clone {
    /// The error type of taking a dual.
    type DualError: Error;

    /// Returns a reference to the underlying abstract polytope.
    fn abs(&self) -> &Abstract;

    /// Returns a mutable reference to the underlying abstract polytope.
    fn abs_mut(&mut self) -> &mut Abstract;

    /// Returns the underlying abstract polytope.
    fn into_abs(self) -> Abstract;

    /// Returns whether this is a nullitope.
    fn is_nullitope(&self) -> bool {
        self.rank() == 0
    }

    /// Returns a map from the elements in a polytope to the index of one of its
    /// vertices. Does not map the minimal element anywhere.
    fn vertex_map(&self) -> ElementMap<usize> {
        // Maps every element of the polytope to one of its vertices.
        let mut vertex_map = ElementMap::new();
        vertex_map.push(Vec::new());

        // Vertices map to themselves.
        if self.rank() != 0 {
            vertex_map.push((0..self.vertex_count()).collect());
        }

        // Every other element maps to the vertex of any subelement.
        for (r, elements) in self.ranks().iter().enumerate().skip(2) {
            vertex_map.push(
                elements
                    .iter()
                    .map(|el| vertex_map[r - 1][el.subs[0]])
                    .collect(),
            );
        }

        vertex_map
    }

    /// Returns the indices of a Petrial polygon in cyclic order, or `None` if
    /// it self-intersects.
    fn petrie_polygon_vertices(&mut self, flag: Flag) -> Option<Vec<usize>> {
        let rank = self.rank();
        let mut new_flag = flag.clone();
        let first_vertex = flag[0];

        let mut vertices = Vec::new();
        let mut vertex_hash = HashSet::new();

        self.element_sort();

        loop {
            // Applies 0-changes up to (rank-1)-changes in order.
            for idx in 0..rank {
                new_flag.change_mut(self.abs(), idx);
            }

            // If we just hit a previous vertex, we return.
            let new_vertex = new_flag[0];
            if vertex_hash.contains(&new_vertex) {
                return None;
            }

            // Adds the new vertex.
            vertices.push(new_vertex);
            vertex_hash.insert(new_vertex);

            // If we're back to the beginning, we break out of the loop.
            if new_vertex == first_vertex {
                break;
            }
        }

        // We returned to precisely the initial flag.
        if flag == new_flag {
            Some(vertices)
        }
        // The Petrie polygon self-intersects.
        else {
            None
        }
    }

    /// Sorts the subelements and superelements of the entire polytope. This is
    /// usually called before iterating over the flags of the polytope.
    fn element_sort(&mut self) {
        if !self.abs().sorted {
            self.ranks_mut().element_sort()
        }

        self.abs_mut().sorted = true;
    }

    /// Returns an instance of the
    /// [nullitope](https://polytope.miraheze.org/wiki/Nullitope), the unique
    /// polytope of rank &minus;1.
    fn nullitope() -> Self;

    /// Returns an instance of the
    /// [point](https://polytope.miraheze.org/wiki/Point), the unique polytope
    /// of rank 0.
    fn point() -> Self;

    /// Returns an instance of the
    /// [dyad](https://polytope.miraheze.org/wiki/Dyad), the unique polytope of
    /// rank 1.
    fn dyad() -> Self;

    /// Returns an instance of a [polygon](https://polytope.miraheze.org/wiki/Polygon)
    /// with a given number of sides.
    fn polygon(n: usize) -> Self;

    /// Returns the dual of a polytope. Never fails for an abstract polytope. In
    /// case of failing on a concrete polytope, returns the index of a facet
    /// through the inversion center.
    fn try_dual(&self) -> Result<Self, Self::DualError>;

    /// Calls [`Self::try_dual`] and unwraps the result.
    fn dual(&self) -> Self {
        self.try_dual().unwrap()
    }

    /// Builds the dual of a polytope in place. Never fails for an abstract
    /// polytope. In case of failing on a concrete polytope, returns the index
    /// of a facet through the inversion center and does nothing.
    fn try_dual_mut(&mut self) -> Result<(), Self::DualError>;

    /// Calls [`Self::try_dual_mut`] and unwraps the result.
    fn dual_mut(&mut self) {
        self.try_dual_mut().unwrap();
    }

    /// "Appends" a polytope into another, creating a compound polytope. Fails
    /// if the polytopes have different ranks.
    fn comp_append(&mut self, p: Self);

    /// Gets the element with a given rank and index as a polytope, if it exists.
    fn element(&self, rank: usize, idx: usize) -> Option<Self>;

    /// Gets the element figure with a given rank and index as a polytope.
    fn element_fig(&self, rank: usize, idx: usize) -> Result<Option<Self>, Self::DualError> {
        if self.rank() >= (rank - 1) {
            if let Some(mut element_fig) = self.try_dual()?.element(self.rank() - (rank - 1), idx) {
                element_fig.try_dual_mut()?;
                return Ok(Some(element_fig));
            }
        }

        Ok(None)
    }

    /// Gets the section defined by two elements with given ranks and indices as
    /// a polytope, or returns `None` in case no section is defined by these
    /// elements.
    fn section(&self, section: SectionRef) -> Result<Option<Self>, Self::DualError> {
        Ok(
            if let Some(el) = self.element(section.hi_rank, section.hi_idx) {
                el.element_fig(section.lo_rank, section.lo_idx)?
            } else {
                None
            },
        )
    }

    /// Gets the facet associated to the element of a given index as a polytope.
    fn facet(&self, idx: usize) -> Option<Self> {
        let r = self.rank();
        (r != 0).then(|| self.element(r - 1, idx)).flatten()
    }

    /// Gets the verf associated to the element of a given index as a polytope.
    fn verf(&self, idx: usize) -> Result<Option<Self>, Self::DualError> {
        self.element_fig(1, idx)
    }

    /// Builds a compound polytope from a set of components.
    fn compound(components: Vec<Self>) -> Self {
        Self::compound_iter(components.into_iter())
    }

    /// Builds a compound polytope from an iterator over components.
    fn compound_iter<U: Iterator<Item = Self>>(mut components: U) -> Self {
        if let Some(mut p) = components.next() {
            for q in components {
                p.comp_append(q);
            }
            p
        } else {
            Self::nullitope()
        }
    }

    /// Builds a Petrial in place. Returns `true` if successful. Does not modify
    /// the original polytope otherwise.
    fn petrial_mut(&mut self) -> bool;

    /// Builds the Petrial of a polytope. Returns `None` if the polytope is not
    /// 3D, or if its Petrial is not a valid polytope.
    fn petrial(&self) -> Option<Self> {
        let mut clone = self.clone();
        clone.petrial_mut().then(|| clone)
    }

    /// Builds a Petrie polygon from the first flag of the polytope. Returns
    /// `None` if this Petrie polygon is invalid.
    fn petrie_polygon(&mut self) -> Option<Self> {
        self.petrie_polygon_with(self.first_flag())
    }

    /// Builds a Petrie polygon from a given flag of the polytope. Returns
    /// `None` if this Petrie polygon is invalid.
    fn petrie_polygon_with(&mut self, flag: Flag) -> Option<Self>;

    /// Returns the first [`Flag`] of a polytope. This is the flag built when we
    /// start at the maximal element and repeatedly take the first subelement.
    fn first_flag(&self) -> Flag {
        let rank = self.rank();
        let mut flag = Flag::with_capacity(rank + 1);
        let mut idx = 0;
        flag.push(0);

        for r in 0..rank {
            idx = self.get_element(r, idx).unwrap().sups[0];
            flag.push(idx);
        }

        flag
    }

    /// Returns the first [`OrientedFlag`] of a polytope. This is the flag built
    /// when we start at the maximal element and repeatedly take the first
    /// subelement.
    fn first_oriented_flag(&self) -> OrientedFlag {
        self.first_flag().into()
    }

    /// Returns an iterator over all [`Flag`]s of a polytope.
    fn flags(&self) -> FlagIter<'_> {
        FlagIter::new(self.abs())
    }

    /// Returns an iterator over all [`OrientedFlag`]s of a polytope.
    fn flag_events(&self) -> OrientedFlagIter<'_> {
        OrientedFlagIter::new(self.abs())
    }

    /// Returns the omnitruncate of a polytope.
    fn omnitruncate(&self) -> Self;

    /// Builds a [duopyramid](https://polytope.miraheze.org/wiki/Pyramid_product)
    /// from two polytopes.
    fn duopyramid(p: &Self, q: &Self) -> Self;

    /// Builds a [duoprism](https://polytope.miraheze.org/wiki/Prism_product)
    /// from two polytopes.
    fn duoprism(p: &Self, q: &Self) -> Self;

    /// Builds a [duotegum](https://polytope.miraheze.org/wiki/Tegum_product)
    /// from two polytopes.
    fn duotegum(p: &Self, q: &Self) -> Self;

    /// Builds a [duocomb](https://polytope.miraheze.org/wiki/Honeycomb_product)
    /// from two polytopes.
    fn duocomb(p: &Self, q: &Self) -> Self;

    /// Builds a [ditope](https://polytope.miraheze.org/wiki/Ditope) of a given
    /// polytope.
    fn ditope(&self) -> Self {
        let mut clone = self.clone();
        clone.ditope_mut();
        clone
    }

    /// Builds a [ditope](https://polytope.miraheze.org/wiki/Ditope) of a given
    /// polytope in place.
    fn ditope_mut(&mut self);

    /// Builds a [hosotope](https://polytope.miraheze.org/wiki/hosotope) of a
    /// given polytope.
    fn hosotope(&self) -> Self {
        let mut clone = self.clone();
        clone.hosotope_mut();
        clone
    }

    /// Builds a [hosotope](https://polytope.miraheze.org/wiki/hosotope) of a
    /// given polytope in place.
    fn hosotope_mut(&mut self);

    /// Attempts to build an [antiprism](https://polytope.miraheze.org/wiki/Antiprism)
    /// based on a given polytope. If it fails, it returns the index of a facet
    /// through the inversion center.
    fn try_antiprism(&self) -> Result<Self, Self::DualError>;

    /// Calls [`Self::try_antiprism`] and unwraps the result.
    fn antiprism(&self) -> Self {
        self.try_antiprism().unwrap()
    }

    /// Determines whether a given polytope is
    /// [orientable](https://polytope.miraheze.org/wiki/Orientability).
    fn orientable(&mut self) -> bool {
        let abs = self.abs_mut();
        abs.element_sort();

        for flag_event in abs.flag_events() {
            if flag_event.non_orientable() {
                return false;
            }
        }

        true
    }

    /// Builds a [pyramid](https://polytope.miraheze.org/wiki/Pyramid) from a
    /// given base.
    fn pyramid(&self) -> Self {
        Self::duopyramid(self, &Self::point())
    }

    /// Builds a [pyramid](https://polytope.miraheze.org/wiki/Pyramid) from a
    /// given base.
    ///
    /// This is slightly more optimal in the case of named polytopes.
    fn pyramid_mut(&mut self) {
        *self = self.pyramid();
    }

    /// Builds a [prism](https://polytope.miraheze.org/wiki/Prism) from a
    /// given base.
    fn prism(&self) -> Self {
        Self::duoprism(self, &Self::dyad())
    }

    /// Builds a [prism](https://polytope.miraheze.org/wiki/Prism) from a
    /// given base.
    ///
    /// This is slightly more optimal in the case of named polytopes.
    fn prism_mut(&mut self) {
        *self = self.prism();
    }

    /// Builds a [tegum](https://polytope.miraheze.org/wiki/Bipyramid) from a
    /// given base.
    fn tegum(&self) -> Self {
        Self::duotegum(self, &Self::dyad())
    }

    /// Builds a [tegum](https://polytope.miraheze.org/wiki/Bipyramid) from a
    /// given base.
    ///
    /// This is slightly more optimal in the case of named polytopes.
    fn tegum_mut(&mut self) {
        *self = self.tegum();
    }

    /// Takes the [pyramid product](https://polytope.miraheze.org/wiki/Pyramid_product)
    /// of an iterator over polytopes.
    fn multipyramid<'a, U: Iterator<Item = &'a Self>>(mut factors: U) -> Self
    where
        Self: 'a,
    {
        if let Some(init) = factors.next().cloned() {
            factors.fold(init, |p, q| Self::duopyramid(&p, q))
        } else {
            Self::nullitope()
        }
    }

    /// Takes the [prism product](https://polytope.miraheze.org/wiki/Prism_product)
    /// of an iterator over polytopes.
    fn multiprism<'a, U: Iterator<Item = &'a Self>>(mut factors: U) -> Self
    where
        Self: 'a,
    {
        if let Some(init) = factors.next().cloned() {
            factors.fold(init, |p, q| Self::duoprism(&p, q))
        } else {
            Self::point()
        }
    }

    /// Takes the [tegum product](https://polytope.miraheze.org/wiki/Tegum_product)
    /// of an iterator over polytopes.
    fn multitegum<'a, U: Iterator<Item = &'a Self>>(mut factors: U) -> Self
    where
        Self: 'a,
    {
        if let Some(init) = factors.next().cloned() {
            factors.fold(init, |p, q| Self::duotegum(&p, q))
        } else {
            Self::point()
        }
    }

    /// Takes the [comb product](https://polytope.miraheze.org/wiki/Comb_product)
    /// of an iterator over polytopes.
    fn multicomb<'a, U: Iterator<Item = &'a Self>>(mut factors: U) -> Self
    where
        Self: 'a,
    {
        if let Some(init) = factors.next().cloned() {
            factors.fold(init, |p, q| Self::duocomb(&p, q))
        }
        // There's no sensible way to take an empty comb product, so we just
        // make it a nullitope for simplicity.
        else {
            Self::nullitope()
        }
    }

    /// Builds a [simplex](https://polytope.miraheze.org/wiki/Simplex) with a
    /// given rank.
    fn simplex(rank: usize) -> Self {
        Self::multipyramid(iter::repeat(&Self::point()).take(rank))
    }

    /// Builds a [hypercube](https://polytope.miraheze.org/wiki/Hypercube) with
    /// a given rank.
    fn hypercube(rank: usize) -> Self {
        if rank == 0 {
            Self::nullitope()
        } else {
            Self::multiprism(iter::repeat(&Self::dyad()).take(rank - 1))
        }
    }

    /// Builds an [orthoplex](https://polytope.miraheze.org/wiki/Orthoplex) with
    /// a given rank.
    fn orthoplex(rank: usize) -> Self {
        if rank == 0 {
            Self::nullitope()
        } else {
            Self::multitegum(iter::repeat(&Self::dyad()).take(rank - 1))
        }
    }
}
