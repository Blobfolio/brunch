/*!
# Brunch: Stats
*/

pub(super) mod history;

use crate::{
	Abacus,
	BrunchError,
	MIN_SAMPLES,
};
use dactyl::{
	NiceFloat,
	NicePercent,
	total_cmp,
	traits::SaturatingFrom,
};
use std::{
	cmp::Ordering,
	num::NonZeroU32,
	time::Duration,
};



#[derive(Debug, Clone, Copy)]
/// # Runtime Stats!
pub(crate) struct Stats {
	/// # Total Samples.
	total: NonZeroU32,

	/// # Valid Samples.
	valid: NonZeroU32,

	/// # Standard Deviation.
	deviation: f64,

	/// # Mean Duration of Valid Samples.
	mean: f64,
}

impl TryFrom<Vec<Duration>> for Stats {
	type Error = BrunchError;
	fn try_from(samples: Vec<Duration>) -> Result<Self, Self::Error> {
		let total = u32::saturating_from(samples.len());
		let total = NonZeroU32::new(total).ok_or(BrunchError::TooSmall(total))?;
		if total < MIN_SAMPLES {
			return Err(BrunchError::TooSmall(total.get()));
		}

		// Crunch!
		let mut calc = Abacus::from(samples);
		calc.prune_outliers();

		let valid = u32::saturating_from(calc.len());
		let valid = NonZeroU32::new(valid).ok_or(BrunchError::TooWild)?;
		if valid < MIN_SAMPLES {
			return Err(BrunchError::TooWild);
		}

		let mean = calc.mean();
		let deviation = calc.deviation();

		// Done!
		let out = Self { total, valid, deviation, mean };
		if out.is_valid() { Ok(out) }
		else { Err(BrunchError::Overflow) }
	}
}

impl Stats {
	/// # Deviation?
	///
	/// This method is used to compare a past run with this (present) run to
	/// see if it deviates in a meaningful way.
	///
	/// In practice, that means the absolute difference is greater than one
	/// percent, and the old mean falls outside this run's valid range.
	pub(crate) fn is_deviant(self, other: Self) -> Option<String> {
		let lo = self.deviation.mul_add(-2.0, self.mean);
		let hi = self.deviation.mul_add(2.0, self.mean);
		if total_cmp!((other.mean) < lo) || total_cmp!((other.mean) > hi) {
			let (color, sign, diff) = match self.mean.total_cmp(&other.mean) {
				Ordering::Less => (92, "-", other.mean - self.mean),
				Ordering::Equal => return None,
				Ordering::Greater => (91, "+", self.mean - other.mean),
			};

			return Some(format!(
				"\x1b[{}m{}{}\x1b[0m",
				color,
				sign,
				NicePercent::from(diff / other.mean),
			));
		}

		None
	}

	/// # Nice Mean.
	///
	/// Return the mean rescaled to the most appropriate unit.
	pub(crate) fn nice_mean(self) -> String {
		let (mean, unit) =
			if total_cmp!((self.mean) < 0.000_001) {
				(self.mean * 1_000_000_000.0, "ns")
			}
			else if total_cmp!((self.mean) < 0.001) {
				(self.mean * 1_000_000.0, "\u{3bc}s")
			}
			else if total_cmp!((self.mean) < 1.0) {
				(self.mean * 1_000.0, "ms")
			}
			else {
				(self.mean, "s ")
			};

		format!("\x1b[0;1m{} {unit}\x1b[0m", NiceFloat::from(mean).precise_str(2))
	}

	/// # Samples.
	///
	/// Return the valid/total samples.
	pub(crate) const fn samples(self) -> (NonZeroU32, NonZeroU32) {
		(self.valid, self.total)
	}

	/// # Is Valid?
	fn is_valid(self) -> bool {
		MIN_SAMPLES <= self.valid &&
		self.valid <= self.total &&
		self.deviation.is_finite() &&
		total_cmp!((self.deviation) >= 0.0) &&
		self.mean.is_finite() &&
		total_cmp!((self.mean) >= 0.0)
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_stats_valid() {
		let mut stat = Stats {
			total: NonZeroU32::new(2500).unwrap(),
			valid: NonZeroU32::new(2496).unwrap(),
			deviation: 0.000_000_123,
			mean: 0.000_002_2,
		};

		assert!(stat.is_valid(), "Stat should be valid.");

		stat.total = NonZeroU32::new(100).unwrap();
		assert!(! stat.is_valid(), "Insufficient total.");

		stat.valid = NonZeroU32::new(100).unwrap();
		assert!(stat.is_valid(), "Stat should be valid.");

		stat.valid = NonZeroU32::new(30).unwrap();
		assert!(! stat.is_valid(), "Insufficient samples.");

		stat.valid = NonZeroU32::new(100).unwrap();
		assert!(stat.is_valid(), "Stat should be valid.");

		stat.deviation = f64::NAN;
		assert!(! stat.is_valid(), "NaN deviation.");
		stat.deviation = -0.003;
		assert!(! stat.is_valid(), "Negative deviation.");

		stat.deviation = 0.003;
		assert!(stat.is_valid(), "Stat should be valid.");

		stat.mean = f64::NAN;
		assert!(! stat.is_valid(), "NaN mean.");
		stat.mean = -0.003;
		assert!(! stat.is_valid(), "Negative mean.");
	}
}
