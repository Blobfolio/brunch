/*!
# Brunch: Math
*/

use crate::util;
use std::{
	cmp::Ordering,
	time::Duration,
};



#[derive(Debug)]
/// # Abacus.
///
/// This struct wraps a set of durations (from i.e. a bench run), providing
/// methods to calculate relevant metrics like mean, standard deviation,
/// quantiles, etc.
///
/// (This is basically where the stats from `Stats` come from.)
pub(crate) struct Abacus {
	set: Vec<f64>,
	len: usize,
	unique: usize,
	total: f64,
}

impl From<Vec<Duration>> for Abacus {
	fn from(src: Vec<Duration>) -> Self {
		let set: Vec<f64> = src.iter().map(Duration::as_secs_f64).collect();
		Self::from(set)
	}
}

impl From<Vec<f64>> for Abacus {
	fn from(mut set: Vec<f64>) -> Self {
		// Negative and abnormal values make no sense for our purposes, so
		// let's pre-emptively strip them out.
		set.retain(|f|
			match f.total_cmp(&0.0) {
				Ordering::Equal => true,
				Ordering::Greater if f.is_normal() => true,
				_ => false,
			}
		);

		// Everything from here on out requires a sorted set, so let's take
		// care of that now.
		set.sort_by(f64::total_cmp);

		// Pre-calculate some useful totals.
		let len = set.len();
		let unique = count_unique(&set);
		let total = set.iter().sum();

		// Done!
		Self { set, len, unique, total }
	}
}

impl Abacus {
	/// # Is Empty?
	const fn is_empty(&self) -> bool { self.len == 0 }

	/// # Length.
	pub(crate) const fn len(&self) -> usize { self.len }

	#[allow(clippy::cast_precision_loss)]
	/// # Float Length.
	const fn f_len(&self) -> f64 { self.len as f64 }
}

impl Abacus {
	/// # Standard Deviation.
	///
	/// Note: this uses the _n_ rather than _n+1_ approach.
	pub(crate) fn deviation(&self) -> f64 {
		if self.is_empty() || self.unique == 1 { return 0.0; }
		let mean = self.mean();
		let squares: Vec<f64> = self.set.iter()
			.map(|n| (mean - *n).powi(2))
			.collect();
		let sum: f64 = squares.iter().sum();
		(sum / self.f_len()).sqrt()
	}

	/// # Maximum Value.
	pub(crate) fn max(&self) -> f64 {
		if self.is_empty() { 0.0 }
		else { self.set[self.len() - 1] }
	}

	/// # Mean.
	pub(crate) fn mean(&self) -> f64 {
		if self.is_empty() { 0.0 }
		else if self.unique == 1 { self.set[0] }
		else { self.total / self.f_len() }
	}

	/// # Minimum Value.
	pub(crate) fn min(&self) -> f64 {
		if self.is_empty() { 0.0 }
		else { self.set[0] }
	}
}

impl Abacus {
	/// # Prune Outliers.
	///
	/// This calculates an IQR using the 5th and 95th quantiles (fuzzily), and
	/// removes entries below the lower boundary or above the upper one, using
	/// a multiplier of `1.5`.
	pub(crate) fn prune_outliers(&mut self) {
		if 1 < self.unique && 0.0 < self.deviation() {
			let q1 = self.ideal_quantile(0.05);
			let q3 = self.ideal_quantile(0.95);
			let iqr = q3 - q1;

			// Low and high boundaries.
			let lo = iqr.mul_add(-1.5, q1);
			let hi = iqr.mul_add(1.5, q3);

			// Remove outliers.
			self.set.retain(|&s| util::float_le(lo, s) && util::float_le(s, hi));

			// Recalculate totals if the length changed.
			let len = self.set.len();
			if len != self.len {
				self.len = len;
				self.unique = count_unique(&self.set);
				self.total = self.set.iter().sum();
			}
		}
	}
}

impl Abacus {
	/// # Count Above.
	///
	/// Return the total number of entries with values larger than the target.
	fn count_above(&self, num: f64) -> usize {
		self.set.iter()
			.rev()
			.take_while(|n| util::float_gt(**n, num))
			.count()
	}

	/// # Count Below.
	///
	/// Return the total number of entries with values lower than the target.
	fn count_below(&self, num: f64) -> usize {
		self.set.iter()
			.take_while(|n| util::float_lt(**n, num))
			.count()
	}

	#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
	/// # Quantile.
	///
	/// Return the quantile at the corresponding percentage. Values are clamped
	/// to the set's minimum and maximum, but will always correspond to a value
	/// that is actually in the set.
	fn quantile(&self, phi: f64) -> f64 {
		if self.is_empty() { 0.0 }
		else if phi <= 0.0 { self.min() }
		else if phi >= 1.0 { self.max() }
		else if self.len == 1 || self.unique == 1 { self.set[0] }
		else {
			// Find the absolute middle of the set.
			let target = (phi * self.f_len()).round() as usize;
			if target == 0 { self.min() }
			else if target >= self.len - 1 { self.max() }
			else {
				// The number of entries below and above our starting point.
				// Since we mathed this guess, this serves as the "ideal"
				// reference distribution.
				let target_below = target;
				let target_above = self.len.saturating_sub(target + 1);

				// Start with our best-guess value.
				let mut out = self.set[target];
				let mut diff = quantile_diff(
					self.count_below(out),
					self.count_above(out),
					target_below,
					target_above,
				);

				// See if lower values get us closer.
				let mut last = self.set[target];
				while let Some(other) = self.step_down(last) {
					let diff2 = quantile_diff(
						self.count_below(other),
						self.count_above(other),
						target_below,
						target_above,
					);
					if diff2 < diff {
						last = other;
						out = other;
						diff = diff2;
					}
					else { break; }
				}

				// See if higher values get us closer.
				last = self.set[target];
				while let Some(other) = self.step_up(last) {
					let diff2 = quantile_diff(
						self.count_below(other),
						self.count_above(other),
						target_below,
						target_above,
					);
					if diff2 < diff {
						last = other;
						out = other;
						diff = diff2;
					}
					else { break; }
				}

				out
			}
		}
	}

	/// # Idealized Quantile.
	///
	/// Return the quantile at the corresponding percentage. Unlike `Abacus::quantile`,
	/// the result may not actually be present in the set. (Sparse entries are
	/// smoothed out to provide an "idealized" representation of where the cut
	/// would fall if the data were better.)
	///
	/// This was inspired by the [`quantogram`](https://crates.io/crates/quantogram) crate's `fussy_quantile`
	/// calculations, but wound up much simpler because we have only a singular
	/// use case to worry about.
	fn ideal_quantile(&self, phi: f64) -> f64 {
		if self.is_empty() { 0.0 }
		else if phi <= 0.0 { self.min() }
		else if phi >= 1.0 { self.max() }
		else if self.len == 1 || self.unique == 1 { self.set[0] }
		else {
			let epsilon = 1.0 / (2.0 * self.f_len());
			let quantile = self.quantile(phi);
			if quantile == 0.0 || phi <= 1.5 * epsilon || phi >= epsilon.mul_add(-1.5, 1.0) {
				quantile
			}
			else {
				let lo = self.quantile(phi - epsilon);
				let hi = self.quantile(phi + epsilon);

				let lo_diff = quantile - lo;
				let hi_diff = hi - quantile;

				if lo_diff >= hi_diff * 2.0 {
					(lo + quantile) / 2.0
				}
				else if hi_diff >= lo_diff * 2.0 {
					(hi + quantile) / 2.0
				}
				else { 0.0 }
			}
		}
	}

	/// # Step Down.
	///
	/// Return the largest entry in the set with a value lower than the target,
	/// if any.
	fn step_down(&self, num: f64) -> Option<f64> {
		let pos = self.set.iter().position(|n| util::float_eq(*n, num))?;
		if 0 < pos { Some(self.set[pos - 1]) }
		else { None }
	}

	/// # Step Up.
	///
	/// Return the smallest entry in the set with a value larger than the
	/// target, if any.
	fn step_up(&self, num: f64) -> Option<f64> {
		let pos = self.set.iter().rposition(|n| util::float_eq(*n, num))?;
		if pos + 1 < self.len { Some(self.set[pos + 1]) }
		else { None }
	}
}



/// # Count Unique.
///
/// This returns the number of unique entries in a set. It isn't particularly
/// efficient, but won't run more than twice per benchmark, so should be fine.
///
/// Note: values must be pre-sorted.
fn count_unique(src: &[f64]) -> usize {
	let mut unique = src.to_vec();
	unique.dedup_by(|a, b| util::float_eq(*a, *b));
	unique.len()
}

/// # Distance Above and Below.
///
/// This averages the absolute distance between the below counts and above
/// counts. An ideal distribution would return `0.0`.
fn quantile_diff(below: usize, above: usize, ref_below: usize, ref_above: usize) -> f64 {
	let below = below.abs_diff(ref_below);
	let above = above.abs_diff(ref_above);

	dactyl::int_div_float(below + above, 2).unwrap_or_default()
}



#[cfg(test)]
mod tests {
	use super::*;
	use quantogram::Quantogram;

	/// # Basic Test Set.
	fn t_set() -> Vec<f64> {
		vec![
			1.0, 1.0, 1.0,
			1.8, 1.8,
			1.9, 1.9, 1.9, 1.9,
			2.0, 2.0, 2.0, 2.0,
			2.1, 2.1,
			2.2, 2.2,
			2.3, 2.3,
			2.4, 2.4,
			3.0, 3.0, 3.0,
		]
	}

	#[test]
	/// # Compare Metrics.
	///
	/// This uses the third-party `Quantogram` struct to sanity-check the
	/// metrics produced by `Abacus`.
	///
	/// The two structs won't _always_ agree with one another due to the
	/// fickleness of Rust floats, but they _should_ come up with identical
	/// answers in regards to our basic `t_set` data.
	fn t_nanos() {
		let nanos = Abacus::from(t_set());
		let mut q = Quantogram::new();
		q.add_unweighted_samples(t_set().iter());

		assert_eq!(nanos.min(), q.min().unwrap(), "Min.");
		assert_eq!(nanos.max(), q.max().unwrap(), "Max.");
		assert_eq!(nanos.mean(), q.mean().unwrap(), "Mean.");
		assert_eq!(nanos.deviation(), q.stddev().unwrap(), "Standard deviation.");
		assert_eq!(nanos.quantile(0.5), q.quantile(0.5).unwrap(), "Median.");
		assert_eq!(
			nanos.ideal_quantile(0.05),
			q.fussy_quantile(0.05, 2.0).unwrap(),
			"Fussy 5%."
		);
		assert_eq!(
			nanos.ideal_quantile(0.95),
			q.fussy_quantile(0.95, 2.0).unwrap(),
			"Fussy 95%."
		);
	}
}
