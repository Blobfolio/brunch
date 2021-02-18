/*!
# Brunch: Stats
*/

use serde::{
	Serialize,
	Deserialize,
};
use std::time::Duration;



#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
/// # Stats.
///
/// This is a simple struct to hold the total number of samples and time spent,
/// along with a semi-statistical average and fit-worthiness value.
///
/// The how of the average and fit calculations borrow heavily from [`easybench`](https://crates.io/crates/easybench)
/// as its approach works quite well!
pub struct Stats {
	pub(crate) iters: usize,
	pub(crate) time: Duration,
	pub(crate) avg: f64,
	pub(crate) fit: f64,
}

#[allow(clippy::similar_names)]
impl From<&[(usize, Duration)]> for Stats {
	#[allow(clippy::suspicious_operation_groupings)] // You don't know me.
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

		// Short circuit.
		let len = src.len() as f64;
		if len < 2.0 { return out; }

		// Recast to floats to prevent overflow-type issues with integer division.
		let sum_a = sum_a as f64;
		let sum_b = sum_b.as_nanos() as f64;

		let (sq_a, sq_b, prod) = src.iter()
			.fold((0.0_f64, 0.0_f64, 0.0_f64), |(ta, tb, tp), (a, b)| {
				let a = *a as f64;
				let b = b.as_nanos() as f64;
				(
					a.mul_add(a, ta), // Sum of A squares.
					b.mul_add(b, tb), // Sum of B squares.
					a.mul_add(b, tp)  // Sum of A*B.
				)
			});

		// Now the math.
		let ncovar = prod - ((sum_a * sum_b) / len);
		let nxvar = sq_a - ((sum_a * sum_a) / len);
		let nyvar = sq_b - ((sum_b * sum_b) / len);

		// Save the values.
		out.fit = (ncovar * ncovar) / (nxvar * nyvar);
		out.avg = ncovar / nxvar;

		out
	}
}
