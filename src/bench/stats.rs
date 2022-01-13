/*!
# Brunch: Stats
*/

use dactyl::traits::SaturatingFrom;
use num_traits::cast::FromPrimitive;
use serde::{
	Serialize,
	Deserialize,
};
use std::time::Duration;



#[doc(hidden)]
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
/// # Stats.
///
/// This is a simple struct to hold the total number of samples and time spent,
/// along with a semi-statistical average and fit-worthiness value.
///
/// The how of the average and fit calculations borrow heavily from [`easybench`](https://crates.io/crates/easybench)
/// as its approach works quite well!
///
/// This is triggered automatically when using the [`benches`] macro; it is
/// not intended to be called manually.
pub struct Stats {
	pub(crate) iters: usize,
	pub(crate) time: Duration,
	pub(crate) avg: f64,
	pub(crate) fit: f64,
}

#[allow(clippy::similar_names)]
impl From<&[(usize, Duration)]> for Stats {
	fn from(src: &[(usize, Duration)]) -> Self {
		// Sums.
		let (sum_a, sum_b) = src.iter()
			.fold((0_usize, Duration::default()), |(ta, tb), (a, b)| (ta + a, tb + *b));

		let mut out = Self {
			iters: sum_a,
			time: sum_b,
			avg: f64::NAN,
			fit: f64::NAN,
		};

		// Length should fit in a u32.
		let len = u32::saturating_from(src.len());
		if len < 2 { return out; }

		// Convert to an f64.
		let len = f64::from(len);

		// Recast to floats to prevent overflow-type issues with integer division.
		let sum_a = f64::from_usize(sum_a).unwrap_or_default();
		let sum_b = f64::from_u128(sum_b.as_nanos()).unwrap_or_default();

		if sum_a > 0.0 && sum_b > 0.0 {
			let (sq_a, sq_b, prod) = src.iter()
				.fold((0.0_f64, 0.0_f64, 0.0_f64), |(ta, tb, tp), (a, b)| {
					let a = f64::from_usize(*a).unwrap_or_default();
					let b = f64::from_u128(b.as_nanos()).unwrap_or_default();
					(
						a.mul_add(a, ta), // Sum of A squares.
						b.mul_add(b, tb), // Sum of B squares.
						a.mul_add(b, tp)  // Sum of A*B.
					)
				});

			// Now the math.
			let ncovar = prod - ((sum_a * sum_b) / len);
			let nxvar = sq_a - (sum_a.powi(2) / len);
			let nyvar = sq_b - (sum_b.powi(2) / len);

			// Save the values.
			out.fit = ncovar.powi(2) / (nxvar * nyvar);
			out.avg = ncovar / nxvar;
		}

		out
	}
}
