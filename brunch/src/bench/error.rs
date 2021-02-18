/*!
# Brunch: Error
*/

use std::fmt;



#[derive(Debug, Copy, Clone)]
/// # Obligatory Error Enum.
pub enum BenchError {
	/// High Background Noise.
	Inconsistent,
	/// No Callback Specified.
	MissingCallback,
	/// Completed Too Fast.
	TooFast,
	/// Took Too Long.
	TooSlow,
}

impl BenchError {
	#[must_use]
	/// # As Str.
	pub const fn as_str(&self) -> &str {
		match self {
			Self::Inconsistent => "Results were too inconsistent.",
			Self::MissingCallback => "No callback specified!",
			Self::TooFast => "The bench was too fast.",
			Self::TooSlow => "The bench was too slow.",
		}
	}
}

impl fmt::Display for BenchError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}
