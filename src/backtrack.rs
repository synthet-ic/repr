/*!
This is the backtracking matching engine. It has the same exact capability
as the full NFA simulation, except it is artificially restricted to small
regexes on small inputs because of its memory requirements.

In particular, this is a *bounded* backtracking engine. It retains worst
case linear time by keeping track of the states that it has visited (using a
bitmap). Namely, once a state is visited, it is never visited again. Since a
state is keyed by `(instruction index, input index)`, we have that its time
complexity is `O(mn)` (i.e., linear in the size of the search text).

The backtracking engine can beat out the NFA simulation on small
regexes/inputs because it doesn't have to keep track of multiple copies of
the capture groups. In benchmarks, the backtracking engine is roughly twice
as fast as the full NFA simulation. Note though that its performance doesn't
scale, even if you're willing to live with the memory requirements. Namely,
the bitset has to be zeroed on each execution, which becomes quite expensive
on large bitsets.
*/
/// Sets the matching engine to use a bounded backtracking engine no
/// matter what optimizations are possible.
///
/// One must use this with care, since the bounded backtracking engine
/// uses memory proportion to `len(regex) * len(context)`.
///
/// This overrides whatever was previously set via the `automatic` or
/// `nfa` methods.

use crate::context::Context;
use crate::exec::ProgramCache;
use crate::repr::Integral;
use crate::program::{Index, Program, Inst};

type Bits = u32;

const BIT_SIZE: usize = 32;
const MAX_SIZE_BYTES: usize = 256 * (1 << 10); // 256 KB

/// Returns true iff the given regex and input should be executed by this
/// engine with reasonable memory usage.
pub fn should_exec(num_insts: usize, text_len: usize) -> bool {
    // Total memory usage in bytes is determined by:
    //
    //   ((len(insts) * (len(input) + 1) + bits - 1) / bits) * (size_of(u32))
    //
    // The actual limit picked is pretty much a heuristic.
    // See: https://github.com/rust-lang/regex/issues/215
    let size = ((num_insts * (text_len + 1) + BIT_SIZE - 1) / BIT_SIZE) * 4;
    size <= MAX_SIZE_BYTES
}

/// A backtracking matching engine.
#[derive(Debug)]
pub struct Bounded<'a, 'm, 'r, I: Integral> {
    prog: &'r Program<I>,
    context: Context<I>,
    matches: &'m mut [bool],
    m: &'a mut Cache<I>,
}

/// Shared cached state between multiple invocations of a backtracking engine
/// in the same thread.
#[derive(Clone, Debug)]
pub struct Cache<I: Integral> {
    jobs: Vec<Job<I>>,
    visited: Vec<Bits>,
}

impl<I: Integral> Cache<I> {
    /// Create new empty cache for the backtracking engine.
    pub fn new(_prog: &Program<I>) -> Self {
        Cache { jobs: vec![], visited: vec![] }
    }
}

/// A job is an explicit unit of stack space in the backtracking engine.
///
/// The "normal" representation is a single state transition, which corresponds
/// to an NFA state and a character in the input. However, the backtracking
/// engine must keep track of old capture group values. We use the explicit
/// stack to do it.
#[derive(Clone, Copy, Debug)]
struct Job<I: Integral> { ip: Index, at: I }

impl<'a, 'm, 'r, I: Integral> Bounded<'a, 'm, 'r, I> {
    /// Execute the backtracking matching engine.
    ///
    /// If there's a match, `exec` returns `true` and populates the given
    /// captures accordingly.
    pub fn exec(
        prog: &'r Program<I>,
        cache: &ProgramCache<I>,
        matches: &'m mut [bool],
        context: Context<I>,
        start: usize,
        end: usize,
    ) -> bool {
        let mut cache = cache.borrow_mut();
        let cache = &mut cache.backtrack;
        let start = context[start];
        let mut b = Bounded { prog, context, matches, m: cache };
        b.exec_(start, end)
    }

    /// Clears the cache such that the backtracking engine can be executed
    /// on some input of fixed length.
    fn clear(&mut self) {
        // Reset the job memory so that we start fresh.
        self.m.jobs.clear();

        // Now we need to clear the bit state set.
        // We do this by figuring out how much space we need to keep track
        // of the states we've visited.
        // Then we reset all existing allocated space to 0.
        // Finally, we request more space if we need it.
        //
        // This is all a little circuitous, but doing this using unchecked
        // operations doesn't seem to have a measurable impact on performance.
        // (Probably because backtracking is limited to such small
        // inputs/regexes in the first place.)
        let visited_len =
            (self.prog.len() * (self.context.len() + 1) + BIT_SIZE - 1)
                / BIT_SIZE;
        self.m.visited.truncate(visited_len);
        for v in &mut self.m.visited {
            *v = 0;
        }
        if visited_len > self.m.visited.len() {
            let len = self.m.visited.len();
            self.m.visited.reserve_exact(visited_len - len);
            for _ in 0..(visited_len - len) {
                self.m.visited.push(0);
            }
        }
    }

    /// Start backtracking at the given position in the input, but also look
    /// for literal prefixes.
    fn exec_(&mut self, mut at: usize, end: usize) -> bool {
        self.clear();
        // If this is an anchored regex at the beginning of the input, then
        // we're either already done or we only need to try backtracking once.
        if self.prog.is_anchored_start {
            return self.backtrack(at);
        }
        let mut matched = false;
        loop {
            if !self.prog.prefixes.is_empty() {
                at = match self.context.prefix_at(&self.prog.prefixes, at) {
                    None => break,
                    Some(at) => at,
                };
            }
            matched = self.backtrack(at) || matched;
            if matched && self.prog.matches.len() == 1 {
                return true;
            }
            if at >= end {
                break;
            }
            at = self.context[at + 1];
        }
        matched
    }

    /// The main backtracking loop starting at the given input position.
    fn backtrack(&mut self, at: I) -> bool {
        // N.B. We use an explicit stack to avoid recursion.
        // To avoid excessive pushing and popping, most transitions are handled
        // in the `step` helper function, which only pushes to the stack when
        // there's a capture or a branch.
        let mut matched = false;
        self.m.jobs.push(Job { ip: 0, at });
        while let Some(job) = self.m.jobs.pop() {
            if self.step(job.ip, job.at) {
                // Only quit if we're matching one regex.
                // If we're matching a regex set, then mush on and
                // try to find other matches (if we want them).
                if self.prog.matches.len() == 1 {
                    return true;
                }
                matched = true;
            }
        }
        matched
    }

    fn step(&mut self, mut ip: Index, mut at: usize) -> bool {
        loop {
            // This loop is an optimization to avoid constantly pushing/popping
            // from the stack. Namely, if we're pushing a job only to run it
            // next, avoid the push and just mutate `ip` (and possibly `at`)
            // in place.
            if self.has_visited(ip, at) {
                return false;
            }
            match self.prog[ip] {
                Inst::Match(slot) => {
                    if slot < self.matches.len() {
                        self.matches[slot] = true;
                    }
                    return true;
                }
                Inst::Split { goto1, goto2 } => {
                    self.m.jobs.push(Job { ip: goto2, at });
                    ip = goto1;
                }
                Inst::Zero { goto, zero } => {
                    if self.context.is_empty_match(at, zero) {
                        ip = goto;
                    } else {
                        return false;
                    }
                }
                Inst::One { goto, seq } => {
                    if seq == self.context[at] {
                        ip = goto;
                        at = self.context[at + 1];
                    } else {
                        return false;
                    }
                }
                Inst::Interval { goto, interval } => {
                    if interval.has(self.context[at]) {
                        ip = goto;
                        at = self.context[at + 1];
                    } else {
                        return false;
                    }
                }
            }
        }
    }

    fn has_visited(&mut self, ip: Index, at: usize) -> bool {
        let k = ip * (self.context.len() + 1) + at;
        let k1 = k / BIT_SIZE;
        let k2 = usize_to_u32(1 << (k & (BIT_SIZE - 1)));
        if self.m.visited[k1] & k2 == 0 {
            self.m.visited[k1] |= k2;
            false
        } else {
            true
        }
    }
}

fn usize_to_u32(n: usize) -> u32 {
    if (n as u64) > (u32::MAX as u64) {
        panic!("BUG: {} is too big to fit into u32", n)
    }
    n as u32
}
