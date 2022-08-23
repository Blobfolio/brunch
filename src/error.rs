/*!
# Brunch
*/

use dactyl::NiceU64;
use std::fmt;



#[derive(Debug, Clone, Copy)]
/// # Error.
///
/// This enum serves as the custom error type for `Brunch`.
pub enum BrunchError {
	/// # Duplicate name.
	DupeName,

	/// # No benches were specified.
	NoBench,

	/// # A bench was missing a [`Bench::run`](crate::Bench::run)-type call.
	NoRun,

	/// # General math failure. (Floats aren't fun.)
	Overflow,

	/// # The benchmark completed too quickly to analyze.
	TooFast,

	/// # Not enough samples were collected to analyze.
	TooSmall(usize),

	/// # The samples were too chaotic to analyze.
	TooWild,
}

impl std::error::Error for BrunchError {}

impl fmt::Display for BrunchError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DupeName => f.write_str("Benchmark names must be unique."),
			Self::NoBench => f.write_str("At least one benchmark is required."),
			Self::NoRun => f.write_str("Missing \x1b[1;96mBench::run\x1b[0m."),
			Self::Overflow => f.write_str("Unable to crunch the numbers."),
			Self::TooFast => f.write_str("Too fast to benchmark!"),
			Self::TooSmall(n) => write!(
				f, "Insufficient samples collected ({}); try increasing the timeout.",
				NiceU64::from(*n),
			),
			Self::TooWild => f.write_str("Samples too wild to analyze."),
		}
	}
}
