/*!
# Brunch: Bench
*/

use crate::{
	BrunchError,
	History,
	MIN_SAMPLES,
	Stats,
	util,
};
use dactyl::NiceU64;
use std::{
	fmt,
	num::NonZeroUsize,
	time::{
		Duration,
		Instant,
	},
};



#[allow(unsafe_code)]
/// # Safety: This is non-zero.
const DEFAULT_SAMPLES: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2500) };
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const NO_CHANGE: &str = "\x1b[2m---\x1b[0m";



#[derive(Debug, Default)]
/// # Benchmarks.
///
/// This holds a collection of benchmarks. You don't need to interact with this
/// directly when using the [`benches`](crate::benches) macro, but _do_ need to use it if
/// manually constructing the `main()` method.
///
/// Refer to the main documentation for examples.
pub struct Benches(Vec<Bench>);

impl Extend<Bench> for Benches {
	/// # Extend.
	///
	/// Insert [`Bench`]es en-masse.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::{Benches, Bench};
	///
	/// let mut benches = Benches::default();
	/// benches.extend([
	///     Bench::new("String::len").run(|| "Hello World".len()),
	///     Bench::spacer(),
	/// ]);
	/// benches.finish();
	/// ```
	fn extend<T: IntoIterator<Item=Bench>>(&mut self, iter: T) {
		for b in iter { self.push(b); }
	}
}

impl Benches {
	/// # Add Benchmark.
	///
	/// Use this method to push a benchmark to your `Benches` collection. Each
	/// benchmark should be pushed before running [`Benches::finish`].
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::{Benches, Bench};
	///
	/// let mut benches = Benches::default();
	/// benches.push(Bench::new("String::len").run(|| "Hello World".len()));
	/// // Repeat push as needed.
	/// benches.finish();
	/// ```
	pub fn push(&mut self, b: Bench) { self.0.push(b); }

	/// # Finish.
	///
	/// Crunch and print the data!
	///
	/// This method should only be called after all benchmarks have been pushed
	/// to the set.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::{Benches, Bench};
	///
	/// let mut benches = Benches::default();
	/// benches.push(Bench::new("String::len").run(|| "Hello World".len()));
	/// benches.finish();
	/// ```
	pub fn finish(&self) {
		// If there weren't any benchmarks, just print an error.
		if self.0.is_empty() {
			eprintln!("\x1b[1;91mError:\x1b[0m {}", BrunchError::NoBench);
			return;
		}

		// Build the summaries.
		let mut history = History::default();
		let mut summary = Table::default();
		for b in &self.0 {
			summary.push(b, &history);
		}

		// Update the history.
		self.finish_history(&mut history);

		eprintln!("{}", summary);
	}

	/// # Finish: Update History.
	fn finish_history(&self, history: &mut History) {
		// Copy over the values.
		for b in &self.0 {
			if let Some(Ok(s)) = b.stats {
				history.insert(&b.name, s);
			}
		}

		// Save it.
		history.save();
	}
}



#[derive(Debug)]
/// # Benchmark.
///
/// This struct holds a single "bench" you wish to run. See the main crate
/// documentation for more information.
pub struct Bench {
	name: String,
	samples: NonZeroUsize,
	timeout: Duration,
	stats: Option<Result<Stats, BrunchError>>,
}

impl Bench {
	#[must_use]
	/// # New.
	///
	/// Instantiate a new benchmark with a name. The name can be anything, but
	/// is intended to represent the method call itself, like `foo::bar(10)`.
	///
	/// Note: the names should be unique across all benchmarks, as they serve
	/// as the key used when pulling "history". If you have two totally
	/// different benchmarks named the same thing, the run-to-run change
	/// reporting won't make any sense. ;)
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::{NiceU8, NiceU16};
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(0)")
    ///         .run(|| NiceU8::from(0_u8)),
    /// );
    /// ```
	///
	/// ## Panics
	///
	/// This method will panic if the name is empty.
	pub fn new<S>(name: S) -> Self
	where S: AsRef<str> {
		let name = name.as_ref().trim();
		assert!(! name.is_empty(), "Name is required.");

		Self {
			name: name.to_owned(),
			samples: DEFAULT_SAMPLES,
			timeout: DEFAULT_TIMEOUT,
			stats: None,
		}
	}

	#[must_use]
	/// # Spacer.
	///
	/// This will render as a linebreak when printing results, useful if you
	/// want to add visual separation between two different benchmarks.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::{NiceU8, NiceU16};
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(0)")
    ///         .run(|| NiceU8::from(0_u8)),
    ///
    ///     Bench::spacer(),
    ///
    ///     Bench::new("dactyl::NiceU16::from(0)")
    ///         .run(|| NiceU16::from(0_u16)),
    /// );
	/// ```
	pub const fn spacer() -> Self {
		Self {
			name: String::new(),
			samples: DEFAULT_SAMPLES,
			timeout: DEFAULT_TIMEOUT,
			stats: None,
		}
	}

	/// # Is Spacer?
	fn is_spacer(&self) -> bool { self.name.is_empty() }

	#[must_use]
	/// # With Time Limit.
	///
	/// By default, benches stop after reaching 2500 samples or 10 seconds,
	/// whichever comes first.
	///
	/// This method can be used to override the time limit portion of that
	/// equation.
	///
	/// Note: the minimum cutoff time is half a second.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::NiceU8;
	/// use std::time::Duration;
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(0)")
    ///         .with_timeout(Duration::from_secs(1))
    ///         .run(|| NiceU8::from(0_u8))
    /// );
	/// ```
	pub const fn with_timeout(mut self, timeout: Duration) -> Self {
		if timeout.as_millis() < 500 {
			self.timeout = Duration::from_millis(500);
		}
		else { self.timeout = timeout; }
		self
	}

	#[allow(unsafe_code)]
	#[must_use]
	/// # With Sample Limit.
	///
	/// By default, benches stop after reaching 2500 samples or 10 seconds,
	/// whichever comes first.
	///
	/// This method can be used to override the sample limit portion of that
	/// equation.
	///
	/// Generally the default is a good sample size, but if your bench takes a
	/// while to complete, you might want to use this method to shorten it up.
	///
	/// Note: the minimum number of samples is 100, but you should aim for at
	/// least 150-200, because that minimum is applied _after_ outliers have
	/// been removed from the set.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::NiceU8;
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(0)")
    ///         .with_samples(50_000)
    ///         .run(|| NiceU8::from(0_u8))
    /// );
	/// ```
	pub const fn with_samples(mut self, samples: usize) -> Self {
		if samples < MIN_SAMPLES {
			// Safety: ten is non-zero.
			self.samples = unsafe { NonZeroUsize::new_unchecked(MIN_SAMPLES) };
		}
		else {
			// Safety: anything 10+ is also non-zero.
			self.samples = unsafe { NonZeroUsize::new_unchecked(samples) };
		}
		self
	}
}

impl Bench {
	#[must_use]
	/// # Run Benchmark!
	///
	/// Use this method to execute a benchmark for a callback that does not
	/// require any external arguments.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::NiceU8;
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(0)")
    ///         .run(|| NiceU8::from(0_u8))
    /// );
	/// ```
	pub fn run<F, O>(mut self, mut cb: F) -> Self
	where F: FnMut() -> O {
		if self.is_spacer() { return self; }

		let mut times: Vec<Duration> = Vec::with_capacity(self.samples.get());
		let now = Instant::now();

		for _ in 0..self.samples.get() {
			let now2 = Instant::now();
			let _res = util::black_box(cb());
			times.push(now2.elapsed());

			if self.timeout <= now.elapsed() { break; }
		}

		self.stats.replace(Stats::try_from(times));

		self
	}

	#[must_use]
	/// # Run Seeded Benchmark!
	///
	/// Use this method to execute a benchmark for a callback seeded with the
	/// provided value.
	///
	/// For seeds that don't implement `Clone`, use [`Bench::run_seeded_with`]
	/// instead.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::NiceU8;
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(13)")
    ///         .run_seeded(13_u8, |v| NiceU8::from(v))
    /// );
	/// ```
	pub fn run_seeded<F, I, O>(mut self, seed: I, mut cb: F) -> Self
	where F: FnMut(I) -> O, I: Clone {
		if self.is_spacer() { return self; }

		let mut times: Vec<Duration> = Vec::with_capacity(self.samples.get());
		let now = Instant::now();

		for _ in 0..self.samples.get() {
			let seed2 = seed.clone();
			let now2 = Instant::now();
			let _res = util::black_box(cb(seed2));
			times.push(now2.elapsed());

			if self.timeout <= now.elapsed() { break; }
		}

		self.stats.replace(Stats::try_from(times));

		self
	}

	#[must_use]
	/// # Run Callback-Seeded Benchmark!
	///
	/// Use this method to execute a benchmark for a callback seeded with the
	/// result of the provided method.
	///
	/// For seeds that implement `Clone`, use [`Bench::run_seeded`] instead.
	///
	/// ## Examples
	///
	/// ```no_run
	/// use brunch::Bench;
	/// use dactyl::NiceU8;
	///
	/// fn make_num() -> u8 { 13_u8 }
	///
	/// brunch::benches!(
    ///     Bench::new("dactyl::NiceU8::from(13)")
    ///         .run_seeded_with(make_num, |v| NiceU8::from(v))
    /// );
	/// ```
	pub fn run_seeded_with<F1, F2, I, O>(mut self, mut seed: F1, mut cb: F2) -> Self
	where F1: FnMut() -> I, F2: FnMut(I) -> O {
		if self.is_spacer() { return self; }

		let mut times: Vec<Duration> = Vec::with_capacity(self.samples.get());
		let now = Instant::now();

		for _ in 0..self.samples.get() {
			let seed2 = seed();
			let now2 = Instant::now();
			let _res = util::black_box(cb(seed2));
			times.push(now2.elapsed());

			if self.timeout <= now.elapsed() { break; }
		}

		self.stats.replace(Stats::try_from(times));

		self
	}
}



#[derive(Debug, Clone)]
/// # Benchmarking Results.
///
/// This table holds the results of all the benchmarks so they can be printed
/// consistently.
struct Table(Vec<TableRow>);

impl Default for Table {
	fn default() -> Self {
		Self(vec![
			TableRow::Normal(
				"\x1b[1;38;5;13mMethod".to_owned(),
				"Mean".to_owned(),
				"Change".to_owned(),
				"Samples\x1b[0m".to_owned()
			),
			TableRow::Spacer,
		])
	}
}

impl fmt::Display for Table {
	#[allow(clippy::many_single_char_names)]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Maximum column widths.
		let (w1, w2, w3, w4) = self.lens();

		// Pre-generate the full-width spacer content.
		let spacer = format!(
			"\x1b[38;5;5m{}\x1b[0m\n",
			"-".repeat(w1 + w2 + w3 + w4 + 12)
		);

		// Pre-generate padding too. We'll slice this to size each time padding
		// is needed.
		let pad = " ".repeat(w1.max(w2).max(w3).max(w4));

		// Print each line!
		for v in &self.0 {
			let (c1, c2, c3, c4) = v.lens();
			match v {
				TableRow::Normal(a, b, c, d) => writeln!(
					f, "{}{}    {}\x1b[1m{}\x1b[0m    {}{}    {}{}",
					a, &pad[..w1 - c1],
					&pad[..w2 - c2], b,
					&pad[..w3 - c3], c,
					&pad[..w4 - c4], d,
				)?,
				TableRow::Error(a, b) => writeln!(
					f, "{}{}    \x1b[1;38;5;208m{}\x1b[0m",
					a, &pad[..w1 - c1], b,
				)?,
				TableRow::Spacer => f.write_str(&spacer)?,
			}
		}

		Ok(())
	}
}

impl Table {
	/// # Add Row.
	fn push(&mut self, src: &Bench, history: &History) {
		if src.is_spacer() { self.0.push(TableRow::Spacer); }
		else {
			let name = format_name(&src.name);
			match src.stats.unwrap_or(Err(BrunchError::NoRun)) {
				Ok(s) => {
					let time = s.nice_mean();
					let diff = history.get(&src.name)
						.and_then(|h| s.is_deviant(h))
						.unwrap_or_else(|| NO_CHANGE.to_owned());
					let (valid, total) = s.samples();
					let samples = format!(
						"\x1b[2m{}\x1b[0;38;5;5m/\x1b[0;2m{}\x1b[0m",
						NiceU64::from(valid),
						NiceU64::from(total),
					);

					self.0.push(TableRow::Normal(name, time, diff, samples));
				},
				Err(e) => {
					self.0.push(TableRow::Error(name, e));
				}
			}
		}
	}

	/// # Widths.
	fn lens(&self) -> (usize, usize, usize, usize) {
		self.0.iter()
			.fold((0, 0, 0, 0), |acc, v| {
				let v = v.lens();
				(
					acc.0.max(v.0),
					acc.1.max(v.1),
					acc.2.max(v.2),
					acc.3.max(v.3),
				)
			})
	}
}



#[derive(Debug, Clone)]
/// # Table Row.
///
/// This holds the data for a single row. There are a few different variations,
/// but it's pretty straight-forward.
enum TableRow {
	Normal(String, String, String, String),
	Error(String, BrunchError),
	Spacer,
}

impl TableRow {
	/// # Lengths (Widths).
	///
	/// Return the (approximate) printable widths for each column.
	fn lens(&self) -> (usize, usize, usize, usize) {
		match self {
			Self::Normal(a, b, c, d) => (
				util::width(a),
				util::width(b),
				util::width(c),
				util::width(d),
			),
			Self::Error(a, _) => (util::width(a), 0, 0, 0),
			Self::Spacer => (0, 0, 0, 0),
		}
	}
}



#[allow(clippy::option_if_let_else)]
/// # Format Name.
///
/// Style up a benchmark name.
fn format_name(name: &str) -> String {
	// Last opening parenthesis?
	if let Some(pos) = name.rfind('(') {
		// Is there a namespace thing behind it?
		if let Some(pos2) = name[..pos].rfind("::") {
			format!("\x1b[2m{}::\x1b[0m{}", &name[..pos2], &name[pos2 + 2..])
		}
		else {
			format!("\x1b[2m{}\x1b[0m{}", &name[..pos], &name[pos..])
		}
	}
	// Last namespace thing?
	else if let Some(pos) = name.rfind("::") {
		format!("\x1b[2m{}::\x1b[0m{}", &name[..pos], &name[pos + 2..])
	}
	// Leave it boring.
	else { ["\x1b[2m", name, "\x1b[0m"].concat() }
}
