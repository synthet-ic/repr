use std::fmt;
use std::mem;
use std::ops::Deref;
use std::slice;

use crate::interval::Interval;
use crate::repr::{Integral, Zero};

use super::literal::LiteralSearcher;

/// `InstPtr` represents the index of an instruction in a regex program.
pub type InstPtr = usize;

/// Program is a sequence of instructions and various facts about thos
/// instructions.
#[derive(Clone)]
pub struct Program<I: Integral> {
    /// A sequence of instructions that represents an NFA.
    pub insts: Vec<Inst<I>>,
    /// Pointers to each Match instruction in the sequence.
    ///
    /// This is always length 1 unless this program represents a regex set.
    pub matches: Vec<InstPtr>,
    /// A pointer to the start instruction. This can vary depending on how
    /// the program was compiled. For example, programs for use with the DFA
    /// engine have a `.*?` inserted at the beginning of unanchored regular
    /// expressions. The actual starting point of the program is after the
    /// `.*?`.
    pub start: InstPtr,
    /// When true, the program is compiled for DFA matching. For example, this
    /// implies `is_bytes` and also inserts a preceding `.*?` for unanchored
    /// regexes.
    pub is_dfa: bool,
    /// When true, the program matches text in reverse (for use only in the
    /// DFA).
    pub is_reverse: bool,
    /// Whether the regex must match from the start of the input.
    pub is_anchored_start: bool,
    /// Whether the regex must match at the end of the input.
    pub is_anchored_end: bool,
    /// Whether this program contains a Unicode word boundary instruction.
    pub has_unicode_word_boundary: bool,
    /// A possibly empty machine for very quickly matching prefix literals.
    pub prefixes: LiteralSearcher<I>,
    /// A limit on the size of the cache that the DFA is allowed to use while
    /// matching.
    ///
    /// The cache limit specifies approximately how much space we're willing to
    /// give to the state cache. Once the state cache exceeds the size, it is
    /// wiped and all states must be re-computed.
    ///
    /// Note that this value does not impact correctness. It can be set to 0
    /// and the DFA will run just fine. (It will only ever store exactly one
    /// state in the cache, and will likely run very slowly, but it will work.)
    ///
    /// Also note that this limit is *per thread of execution*. That is,
    /// if the same regex is used to search text across multiple threads
    /// simultaneously, then the DFA cache is not shared. Instead, copies are
    /// made.
    pub dfa_size_limit: usize,
}

impl<I: Integral> Program<I> {
    /// Creates an empty instruction sequence. Fields are given default
    /// values.
    pub fn new() -> Self {
        Program {
            insts: vec![],
            matches: vec![],
            start: 0,
            // byte_classes: vec![0; 256],
            is_dfa: false,
            is_reverse: false,
            is_anchored_start: false,
            is_anchored_end: false,
            has_unicode_word_boundary: false,
            prefixes: LiteralSearcher::empty(),
            dfa_size_limit: 2 * (1 << 20),
        }
    }

    // /// If pc is an index to a no-op instruction (like Save), then return the
    // /// next pc that is not a no-op instruction.
    // pub fn skip(&self, mut pc: usize) -> usize {
    //     pc
    // }

    /// Return true if and only if an execution engine at instruction `pc` will
    /// always lead to a match.
    pub fn leads_to_match(&self, pc: usize) -> bool {
        if self.matches.len() > 1 {
            // If we have a regex set, then we have more than one ending
            // state, so leading to one of those states is generally
            // meaningless.
            return false;
        }
        match self[pc] {
            Inst::Match(_) => true,
            _ => false,
        }
    }

    /// Returns true if the current configuration demands that an implicit
    /// `.*?` be prepended to the instruction sequence.
    pub fn needs_dotstar(&self) -> bool {
        self.is_dfa && !self.is_reverse && !self.is_anchored_start
    }

    /// Returns true if this program uses Byte instructions instead of
    /// Char/Range instructions.
    pub fn uses_bytes(&self) -> bool {
        self.is_dfa
    }

    /// Return the approximate heap usage of this instruction sequence in
    /// bytes.
    pub fn approximate_size(&self) -> usize {
        // The only instruction that uses heap space is Ranges (for
        // Unicode codepoint programs) to store non-overlapping codepoint
        // ranges. To keep this operation constant time, we ignore them.
        (self.len() * mem::size_of::<Inst<I>>())
            + (self.matches.len() * mem::size_of::<InstPtr>())
            + (256 * mem::size_of::<u8>())
            + self.prefixes.approximate_size()
    }
}

impl<I: Integral> Deref for Program<I> {
    type Target = [Inst<I>];

    #[cfg_attr(feature = "perf-inline", inline(always))]
    fn deref(&self) -> &Self::Target {
        &*self.insts
    }
}

impl<I: Integral> fmt::Debug for Program<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn with_goto(cur: usize, goto: usize, fmtd: String) -> String {
            if goto == cur + 1 {
                fmtd
            } else {
                format!("{} (goto: {})", fmtd, goto)
            }
        }

        fn visible_byte(b: u8) -> String {
            use std::ascii::escape_default;
            let escaped = escape_default(b).collect::<Vec<u8>>();
            String::from_utf8_lossy(&escaped).into_owned()
        }

        for (pc, inst) in self.iter().enumerate() {
            match *inst {
                Inst::Match(slot) => write!(f, "{:04} Match({:?})", pc, slot)?,
                Inst::Split(ref inst) => {
                    write!(
                        f,
                        "{:04} Split({}, {})",
                        pc, inst.goto1, inst.goto2
                    )?;
                }
                Inst::Zero(ref inst) => {
                    let s = format!("{:?}", inst.look);
                    write!(f, "{:04} {}", pc, with_goto(pc, inst.goto, s))?;
                }
                Inst::One(ref inst) => {
                    let s = format!("{:?}", inst.c);
                    write!(f, "{:04} {}", pc, with_goto(pc, inst.goto, s))?;
                }
                Inst::Interval(ref inst) => {
                    let ranges = format!("{:?}-{:?}", inst.seq.0, inst.seq.1);
                    write!(
                        f,
                        "{:04} {}",
                        pc,
                        with_goto(pc, inst.goto, ranges)
                    )?;
                }
            }
            if pc == self.start {
                write!(f, " (start)")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<'a, I: Integral> IntoIterator for &'a Program<I> {
    type Item = &'a Inst<I>;
    type IntoIter = slice::Iter<'a, Inst<I>>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Inst is an instruction code in a Regex program.
///
/// Regrettably, a regex program either contains Unicode codepoint
/// instructions (Char and Ranges) or it contains byte instructions (Bytes).
/// A regex program can never contain both.
///
/// It would be worth investigating splitting this into two distinct types and
/// then figuring out how to make the matching engines polymorphic over those
/// types without sacrificing performance.
///
/// Other than the benefit of moving invariants into the type system, another
/// benefit is the decreased size. If we remove the `Char` and `Ranges`
/// instructions from the `Inst` enum, then its size shrinks from 32 bytes to
/// 24 bytes. (This is because of the removal of a `Box<[]>` in the `Ranges`
/// variant.) Given that byte based machines are typically much bigger than
/// their Unicode analogues (because they can decode UTF-8 directly), this ends
/// up being a pretty significant savings.
#[derive(Clone)]
pub enum Inst<I: Integral> {
    /// Match indicates that the program has reached a match state.
    ///
    /// The number in the match corresponds to the Nth logical regular
    /// expression in this program. This index is always 0 for normal regex
    /// programs. Values greater than 0 appear when compiling regex sets, and
    /// each match instruction gets its own unique value. The value corresponds
    /// to the Nth regex in the set.
    Match(usize),
    /// Split causes the program to diverge to one of two paths in the
    /// program, preferring goto1 in InstSplit.
    Split(InstSplit),
    /// Zero represents a zero-width assertion in a regex program. A
    /// zero-width assertion does not consume any of the input text.
    Zero(InstZero),
    /// Char requires the regex program to match the character in InstOne at
    /// the current position in the input.
    One(InstOne<I>),
    /// Ranges requires the regex program to match the character at the current
    /// position in the input with one of the ranges specified in InstInterval.
    Interval(InstInterval<I>),
}

impl<I: Integral> Inst<I> {
    /// Returns true if and only if this is a match instruction.
    pub fn is_match(&self) -> bool {
        match *self {
            Inst::Match(_) => true,
            _ => false,
        }
    }
}

/// Representation of the Split instruction.
#[derive(Clone, Debug)]
pub struct InstSplit {
    /// The first instruction to try. A match resulting from following goto1
    /// has precedence over a match resulting from following goto2.
    pub goto1: InstPtr,
    /// The second instruction to try. A match resulting from following goto1
    /// has precedence over a match resulting from following goto2.
    pub goto2: InstPtr,
}

/// Representation of the `Zero` instruction.
#[derive(Clone, Debug)]
pub struct InstZero {
    /// The next location to execute in the program if this instruction
    /// succeeds.
    pub goto: InstPtr,
    /// The type of zero-width assertion to check.
    pub look: Zero,
}

/// Representation of the Char instruction.
#[derive(Clone, Debug)]
pub struct InstOne<I: Integral> {
    /// The next location to execute in the program if this instruction
    /// succeeds.
    pub goto: InstPtr,
    /// The character to test.
    pub c: I,
}

/// Representation of the Ranges instruction.
#[derive(Clone)]
pub struct InstInterval<I: Integral> {
    /// The next location to execute in the program if this instruction
    /// succeeds.
    pub goto: InstPtr,
    /// The set of Unicode scalar value ranges to test.
    pub seq: Interval<I>
}

impl<I: Integral> InstInterval<I> {
    /// Tests whether the given input character matches this instruction.
    pub fn matches(&self, c: I) -> bool {
        // This speeds up the `match_class_unicode` benchmark by checking
        // some common cases quickly without binary search. e.g., Matching
        // a Unicode class on predominantly ASCII text.
        if c < self.seq.0 {
            return false;
        }
        if c <= self.seq.1 {
            return true;
        }
        false
        // self.ranges
        //     .binary_search_by(|r| {
        //         if self.seq.1 < c {
        //             Ordering::Less
        //         } else if self.seq.0 > c {
        //             Ordering::Greater
        //         } else {
        //             Ordering::Equal
        //         }
        //     })
        //     .is_ok()
    }

    /// Return the number of distinct characters represented by all of the
    /// ranges.
    pub fn num_chars(&self) -> usize {
        (1 + (self.seq.1 as u32) - (self.seq.0 as u32)) as usize
    }
}
