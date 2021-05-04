//! A language we can use for debugging.

use crate::lang::name::NameType;

use super::{Language, Name, Options, Prefix};

pub struct Dbg;

impl Prefix for Dbg {}

impl Language for Dbg {
    fn suffix(d: usize, _options: Options) -> String {
        format!("({}D)", d)
    }

    fn pyramid_of<T: NameType>(base: &Name<T>, _options: Options) -> String {
        format!("({}) pyramid", Self::parse(base, Options::default()))
    }

    fn prism_of<T: NameType>(base: &Name<T>, _options: Options) -> String {
        format!("({}) prism", Self::parse(base, Options::default()))
    }

    fn tegum_of<T: NameType>(base: &Name<T>, _options: Options) -> String {
        format!("({}) tegum", Self::parse(base, Options::default()))
    }

    fn simplex(rank: usize, _options: Options) -> String {
        format!("{}-simplex", rank)
    }

    fn hyperblock(rank: usize, _options: Options) -> String {
        format!("{}-hyperblock", rank)
    }

    fn hypercube(rank: usize, _options: Options) -> String {
        format!("{}-hypercube", rank)
    }

    fn orthoplex(rank: usize, _options: Options) -> String {
        format!("{}-orthoplex", rank)
    }

    fn multiproduct<T: NameType>(name: &Name<T>, _options: Options) -> String {
        let (bases, kind) = match name {
            Name::Multipyramid(bases) => (bases, "pyramid"),
            Name::Multiprism(bases) => (bases, "prism"),
            Name::Multitegum(bases) => (bases, "tegum"),
            Name::Multicomb(bases) => (bases, "comb"),
            _ => panic!("Not a product!"),
        };

        let mut str_bases = String::new();

        let (last, bases) = bases.split_last().unwrap();
        for base in bases {
            str_bases.push_str(&format!("({})", Self::parse(base, _options)));
            str_bases.push_str(", ");
        }
        str_bases.push_str(&format!("({})", Self::parse(last, _options)));

        format!("({}) {}-{}", str_bases, bases.len(), kind)
    }
}
