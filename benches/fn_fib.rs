/*!
# Benchmark Demo
*/

use brunch::{
	Bench,
	benches,
};



fn fibonacci_recursive(len: usize) -> Vec<u32> {
	assert!(len > 0, "Length must be non-zero.");

	fn fibonacci_idx(n: usize) -> u32 {
		if n == 0 { 0 }
		else if n < 3 { 1 }
		else {
			fibonacci_idx(n - 1) + fibonacci_idx(n - 2)
		}
	}

	let mut out = Vec::with_capacity(len);
	for n in 0..len {
		out.push(fibonacci_idx(n));
	}

	out
}

fn fibonacci_loop(len: usize) -> Vec<u32> {
	assert!(len > 0, "Length must be non-zero.");

	let mut out = Vec::with_capacity(len);
	if len == 1 { out.push(0); }
	else {
		out.push(0);
		out.push(1);

		for n in 2..len {
			out.push(out[n - 1] + out[n - 2]);
		}
	}

	out
}

benches!(
	Bench::new("fibonacci_recursive(30)")
		.with_samples(1000)
		.run(|| fibonacci_recursive(30_usize)),

	Bench::new("fibonacci_loop(30)")
		.run(|| fibonacci_loop(30_usize)),

	// Logical separation can be achieved thusly.
	Bench::spacer(),

	// Something completely different…
	Bench::new("u64::MAX.checked_ilog10()")
		.run(|| u64::MAX.checked_ilog10()),
);

/*
/// # Manual Main.
///
/// This shows how to achieve the same benchmark using the "inline" approach.
///
/// It isn't necessary for these particular benchmarks, but might be in cases
/// where you want to run code before or after the `benches!` bit.
fn main() {
	benches!(
		inline:

		Bench::new("fibonacci_recursive(30)")
			.with_samples(1000)
			.run(|| fibonacci_recursive(30_usize)),

		Bench::new("fibonacci_loop(30)")
			.run(|| fibonacci_loop(30_usize)),
	)
}
*/
