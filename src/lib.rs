/*!
# Usage

```text
use repr::{Repr, consts::DIGIT};
let re = DIGIT * 4 & '-' & DIGIT * 2 & '-' & DIGIT * 2;
assert!(re.is_match("2014-01-01"));
```

```text
let re = (D * 4)["year"] & '-' & (D * 2)["month"] & '-' & (D * 2)["day"];
let before = "2012-03-14, 2013-01-01 and 2014-07-05";
let after = re.replace_all(before, "$m/$d/$y");
assert_eq!(after, "03/14/2012, 01/01/2013 and 07/05/2014");
```

```text
use repr::{Repr, consts::DIGIT};
let wh = WORD | '-';
let re = (wh | '.') * 1.. & '@' & (wh * 1.. & '.') * 1.. & wh * 2..4;
```
*/

#![feature(pattern)]
#![feature(once_cell)]
#![feature(const_trait_impl)]
#![feature(box_syntax)]
#![feature(try_trait_v2)]
#![feature(derive_const)]
#![feature(const_try)]
#![feature(const_for)]
#![feature(const_box)]
#![feature(const_cmp)]
#![feature(const_discriminant)]
#![feature(const_clone)]
#![feature(const_reverse)]
#![feature(const_slice_index)]
#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(const_refs_to_cell)]
#![feature(const_heap)]
#![feature(const_convert)]
#![feature(core_intrinsics)]
// #![feature(const_iter)]
// #![feature(negative_impls)]
// #![feature(specialization)]
#![feature(step_trait)]

extern crate alloc; 

mod backtrack;
mod compile;
mod context;
mod conversions;
mod exec;
mod interval;
mod operators;
mod options;
mod partition;
mod pattern;
mod pikevm;
mod pool;
mod program;
mod seq;
mod sparse;
mod unicode;
mod wrappers;

pub mod char;
pub mod constants;
pub mod derivative;
pub mod macros;
pub mod repr;

pub use constants::perl::{DIGIT, WORD};
pub use context::Context;
pub use interval::Interval;
pub use partition::Partition;
pub use crate::repr::{Repr, Integral, Zero};
pub use seq::Seq;

// #[test]
// fn datetime() {
//     let year = DIGIT * 4;
//     let month = DIGIT * 2;
//     let day = DIGIT * 2;
//     let re = year & '-' & month & '-' & day;
//     let before = "2012-03-14, 2013-01-01 and 2014-07-05";
//     let after = re.replace_all(before, "$m/$d/$y");
//     assert_eq!(after, "03/14/2012, 01/01/2013 and 07/05/2014");
// }

// #[test]
// fn phone_number() {

// }

// #[test]
// fn email() {

// }
// #[test]
// fn url() {

// }
