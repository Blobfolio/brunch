/*!
# Brunch: Bench
*/

pub(super) mod error;
pub(super) mod history;
pub(super) mod stats;

use crate::{
	BenchError,
	black_box,
	History,
	Stats,
};
use dactyl::{
	NiceU32,
	NicePercent,
};
use num_traits::cast::FromPrimitive;
use std::time::{
	Duration,
	Instant,
};



/// # Benchmark iteration scaling factor.
const ITER_SCALE: f64 = 1.1;



#[derive(Debug)]
/// # Benchmark.
///
/// This struct holds a single "bench" you wish to run. See the main crate
/// documentation for more information.
pub struct Bench {
	namespace: String,
	name: String,
	limit: Duration,
	stats: Option<Stats>,
	last: Option<Stats>,
}

impl Bench {
	/// # New.
	///
	/// Instantiate a new benchmark with a namespace and name. These values
	/// — i.e. `namespace::name` — should be unique for each specific
	/// benchmark or historical comparisons will be muffed.
	///
	/// ## Panics
	///
	/// This method will panic if the namespace or name are empty.
	pub fn new<S>(namespace: S, name: S) -> Self
	where S: AsRef<str> {
		let namespace = namespace.as_ref();
		let name = name.as_ref();

		assert!(! namespace.is_empty() && ! name.is_empty(), "Namespace and name are required.");

		Self {
			namespace: namespace.into(),
			name: name.into(),
			limit: Duration::from_secs(3),
			stats: None,
			last: None,
		}
	}

	#[must_use]
	/// # Set Time Limit.
	///
	/// By default, benches will run for around 3 seconds. This value can be
	/// increased for slow benchmarks, or decreased for fast ones, as needed.
	///
	/// Note: this must be called *before* supplying a callback or it will not
	/// apply.
	pub const fn timed(mut self, time: Duration) -> Self {
		self.limit = time;
		self
	}

	/// # With Callback.
	///
	/// Run a benchmark that does not take any arguments.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::NiceU8;
	/// use std::time::Duration;
	///
	/// brunch::benches!(
    /// Bench::new("dactyl::NiceU8", "from(0)")
    ///     .timed(Duration::from_secs(1))
    ///     .with(|| NiceU8::from(0_u8))
    /// );
	/// ```
	pub fn with<F, O>(mut self, mut cb: F) -> Self
	where F: FnMut() -> O {
		let mut data = Vec::new();
		let bench_start = Instant::now();

		while bench_start.elapsed() < self.limit && data.len() < 4_294_967_295 {
			let iters = iter_count(&data);
			let start = Instant::now();
			for _ in 0..iters { black_box(cb()); }
			data.push((iters, start.elapsed()));
		}

		// Treat the first go as a warmup.
		data.remove(0);
		self.stats = Some(Stats::from(&data[..]));

		self
	}

	/// # With Callback.
	///
	/// Run a benchmark that takes a value by value.
	pub fn with_setup<F, I, O>(mut self, env: I, mut cb: F) -> Self
	where F: FnMut(I) -> O, I: Clone {
		let mut data = Vec::new();
		let bench_start = Instant::now();

		while bench_start.elapsed() < self.limit && data.len() < 4_294_967_295 {
			// Prepare the batch arguments in advance.
			let iters = iter_count(&data);
			let mut xs = std::iter::repeat(env.clone())
				.take(iters)
				.collect::<Vec<I>>();

			let start = Instant::now();
			// There appears to be less overhead from draining a collected Vec
			// than simply ForEaching the Take directly.
			xs.drain(..).for_each(|x| { black_box(cb(x)); });
			data.push((iters, start.elapsed()));
		}

		// Treat the first go as a warmup.
		data.remove(0);
		self.stats = Some(Stats::from(&data[..]));

		self
	}

	/// # With Callback.
	///
	/// Run a benchmark that takes a value by reference.
	pub fn with_setup_ref<F, I, O>(mut self, env: I, mut cb: F) -> Self
	where F: FnMut(&I) -> O, I: Clone {
		//let mut gen_env = move || env.clone();
		let mut data = Vec::new();
		let bench_start = Instant::now();

		while bench_start.elapsed() < self.limit && data.len() < 4_294_967_295 {
			// Prepare the batch arguments in advance.
			let iters = iter_count(&data);
			let xs = std::iter::repeat(&env).take(iters);

			let start = Instant::now();
			// For this case, minimal overhead seems to be found by building an
			// iterator of references rather than simply doing a range loop and
			// chucking &env to the callback directly.
			xs.for_each(|x| { black_box(cb(x)); });
			data.push((iters, start.elapsed()));
		}

		// Treat the first go as a warmup.
		data.remove(0);
		self.stats = Some(Stats::from(&data[..]));

		self
	}

	/// # Update History.
	///
	/// This is triggered automatically when using the `benches!` macro; it is
	/// not intended to be called manually.
	pub(crate) fn history(&mut self, history: &mut History) {
		if let Some(stats) = self.stats {
			self.last = history.insert(
				format!("{}::{}", self.namespace, self.name),
				stats
			);
		}
	}
}



#[doc(hidden)]
#[derive(Debug)]
/// # Benchmark Result.
///
/// This is generated automatically when using the [`benches`] macro; it is
/// not intended to be used directly.
pub struct BenchResult {
	caller: String,
	time: String,
	diff: String,
	error: Option<BenchError>,
}

impl From<&Bench> for BenchResult {
	fn from(src: &Bench) -> Self {
		// The caller.
		let caller = format!("\x1b[2m{}::\x1b[0m{}", src.namespace, src.name);

		// Make sure we have stats to even look at!
		if let Some(stats) = src.stats {
			// Insufficient samples.
			if stats.iters < 50 {
				return Self {
					caller,
					time: String::new(),
					diff: String::new(),
					error: Some(BenchError::TooSlow),
				};
			}
			// Background noise was too high to be meaningful.
			else if stats.fit.is_nan() || stats.fit < 0.95 {
				return Self {
					caller,
					time: String::new(),
					diff: String::new(),
					error: Some(BenchError::Inconsistent),
				};
			}
			// The benchmark was too fast to count.
			else if stats.avg.is_nan() || stats.avg < 0.001 {
				return Self {
					caller,
					time: String::new(),
					diff: String::new(),
					error: Some(BenchError::TooFast),
				};
			}

			// Time.
			let time =
				if stats.avg < 1.0 {
					format_time(stats.avg * 1000.0, "ps")
				}
				else if stats.avg < 1000.0 {
					format_time(stats.avg, "ns")
				}
				else if stats.avg < 1_000_000.0 {
					format_time(stats.avg / 1000.0, "\u{3bc}s") // μs
				}
				else {
					format_time(stats.avg / 1_000_000.0, "ms")
				};

			Self {
				caller,
				time,
				diff: src.last.map_or(String::new(), |h| {
					let (color, sign, diff) =
						if h.avg > stats.avg { ("92", "-", h.avg - stats.avg) }
						else { ("91", "+", stats.avg - h.avg) };

					// Ignore any change within half a percent.
					if diff / h.avg < 0.005 {
						String::from(" \x1b[2m---\x1b[0m  ")
					}
					// Show the difference!
					else {
						format!(
							"\x1b[{}m{}{}\x1b[0m",
							color,
							sign,
							NicePercent::from(diff / h.avg).as_str()
						)
					}
				}),
				error: None,
			}
		}
		// Nothing was run?
		else {
			Self {
				caller,
				time: String::new(),
				diff: String::new(),
				error: Some(BenchError::MissingCallback),
			}
		}
	}
}

impl BenchResult {
	#[must_use]
	/// # Get Part Lengths.
	///
	/// This is used to size the columns from multiple results neatly.
	pub fn lens(&self) -> (usize, usize, usize) {
		if self.error.is_some() {
			(self.caller.len(), 0, 0)
		}
		else {
			(self.caller.len(), self.time.len(), self.diff.len())
		}
	}

	/// # Print!
	///
	/// This will print results, padding each column according to the lengths
	/// passed to it.
	pub fn print(&self, c1: usize, c2: usize, c3: usize) {
		// Errors don't need to worry about spacing.
		if let Some(e) = self.error {
			println!("{}   \x1b[1;38;5;208m{}\x1b[0m", self.caller, e);
			return;
		}

		let (r1, r2, r3) = self.lens();

		// Print the caller.
		print!("{}", self.caller);

		// Pad the caller.
		if r1 < c1 { print!("{}", " ".repeat(c1 - r1)); }

		// Spacer + Timing + Spacer + Difference.
		println!(
			"{}{}{}{}",
			" ".repeat(c2.saturating_sub(r2) + 3),
			self.time,
			" ".repeat(c3.saturating_sub(r3) + 3),
			self.diff
		);
	}
}



/// # Iter Count.
///
/// This will calculate the number of iterations to perform before checking the
/// time. The value will always fall within the `u32` range to make
/// cross-casting safer.
fn iter_count<T>(src: &[T]) -> usize {
	usize::from_f64(
		ITER_SCALE.powi(i32::try_from(src.len()).unwrap_or(i32::MAX)).round()
	)
		.unwrap_or(usize::MAX)
		.min(4_294_967_295 - src.len())
}

/// # Format w/ Unit.
///
/// Give us a nice comma-separated integer with two decimal places and an
/// appropriate unit (running from pico seconds to milliseconds).
fn format_time(time: f64, unit: &str) -> String {
	format!(
		"\x1b[1m{}.{:02} {}\x1b[0m",
		NiceU32::from(u32::from_f64(time.trunc()).unwrap_or_default()).as_str(),
		u32::from_f64(f64::floor(time.fract() * 100.0)).unwrap_or_default(),
		unit
	)
}
