use core::char::from_u32;

use unconst::unconst;

use crate::context::Context;
use crate::interval::Interval;
use crate::derivative::LiteralSearcher;
use crate::repr::{Repr, Integral, Zero};

#[unconst]
impl const Integral for char {
    const MIN: Self = '\x00';
    const MAX: Self = '\u{10FFFF}';
    fn succ(self) -> Self {
        match self {
            '\u{D7FF}' => '\u{E000}',
            c => from_u32((c as u32).checked_add(1).unwrap()).unwrap(),
        }
    }
    fn pred(self) -> Self {
        match self {
            '\u{E000}' => '\u{D7FF}',
            c => from_u32((c as u32).checked_sub(1).unwrap()).unwrap(),
        }
    }
}

#[unconst]
impl Repr<char> {
    /// `.` expression that matches any character except for `\n`. To build an
    /// expression that matches any character, including `\n`, use the `any`
    /// method.
    pub const fn dot() -> Self {
        Self::Or(box Self::Interval(Interval('\0', '\x09')),
                 box Self::Interval(Interval('\x0B', '\u{10FFFF}')))
    }

    // /// `(?s).` expression that matches any character, including `\n`. To build an
    // /// expression that matches any character except for `\n`, then use the
    // /// `dot` method.
    // pub const fn any() -> Self {
    //     Self::Interval(Interval('\0', '\u{10FFFF}'))
    // }
}

/// An abstraction over input used in the matching engines.
impl Context<char> {
    /// Return true if the given empty width instruction matches at the
    /// input position given.
    pub fn is_empty_match(&self, at: usize, look: &Zero) -> bool {
        match look {
            Zero::StartLine => {
                let c = &self[at - 1];
                at == 0 || c == '\n'
            }
            Zero::EndLine => {
                let c = &self[at + 1];
                at == self.len() || c == '\n'
            }
            Zero::StartText => at == 0,
            Zero::EndText => at == self.len(),
            Zero::WordBoundary => {
                let (c1, c2) = (&self[at - 1], &self[at + 1]);
                is_word_char(c1) != is_word_char(c2)
            }
            Zero::NotWordBoundary => {
                let (c1, c2) = (&self[at - 1], &self[at + 1]);
                is_word_char(c1) == is_word_char(c2)
            }
            Zero::WordBoundaryAscii => {
                let (c1, c2) = (&self[at - 1], &self[at + 1]);
                is_word_byte(c1) != is_word_byte(c2)
            }
            Zero::NotWordBoundaryAscii => {
                let (c1, c2) = (&self[at - 1], &self[at + 1]);
                is_word_byte(c1) == is_word_byte(c2)
            }
            Zero::Any => unimplemented!()
        }
    }

    /// Scan the input for a matching prefix.
    pub fn prefix_at(&self, prefixes: &LiteralSearcher<char>, at: usize)
        -> Option<char>
    {
        prefixes.find(&self[at..]).map(|(s, _)| self[at + s])
    }
}

#[unconst]
/// Returns true iff the character is a word character.
///
/// If the character is absent, then false is returned.
pub const fn is_word_char(c: char) -> bool {
    // is_word_character can panic if the Unicode data for \w isn't
    // available. However, our compiler ensures that if a Unicode word
    // boundary is used, then the data must also be available. If it isn't,
    // then the compiler returns an error.
    from_u32(c).map_or(false, regex_syntax::is_word_character)
}

#[unconst]
/// Returns true iff the byte is a word byte.
///
/// If the byte is absent, then false is returned.
pub const fn is_word_byte(c: char) -> bool {
    match from_u32(c) {
        Some(c) if c <= '\u{7F}' => regex_syntax::is_word_byte(c as u8),
        None | Some(_) => false,
    }
}

#[unconst]
/// Returns true iff the character is absent.
#[inline]
pub const fn is_none(c: char) -> bool {
    c == u32::MAX
}
