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
use dactyl::{
	NiceU32,
	traits::SaturatingFrom,
};
use std::{
	fmt,
	hint::black_box,
	num::NonZeroU32,
	time::{
		Duration,
		Instant,
	},
};



/// # Default Sample Count.
const DEFAULT_SAMPLES: NonZeroU32 = NonZeroU32::new(2500).unwrap();

/// # Default Timeout.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// # Markup for No Change "Value".
const NO_CHANGE: &str = "\x1b[2m---\x1b[0m";



#[derive(Debug, Default)]
/// # Benchmarks.
///
/// This holds a collection of benchmarks. You don't need to interact with this
/// directly when using the [`benches`](crate::benches) macro, but can if you
/// want complete control over the whole process.
///
/// ## Examples
///
/// ```no_run
/// use brunch::{Bench, Benches};
/// use std::time::Duration;
///
/// fn main() {
///     // You can do set up, etc., here.
///     eprintln!("Starting benchmarks!");
///
///     // Start a Benches instance.
///     let mut benches = Benches::default();
///
///     // Each Bench needs to be pushed one at a time.
///     benches.push(
///         Bench::new("2_usize.checked_add(2)")
///             .run(|| 2_usize.checked_add(2))
///     );
///
///     // Maybe you want to pause between each benchmark to let the CPU cool?
///     std::thread::sleep(Duration::from_secs(3));
///
///     // Add another Bench.
///     benches.push(
///         Bench::new("200_usize.checked_mul(3)")
///             .run(|| 200_usize.checked_mul(3))
///     );
///
///     // After the last Bench has been added, call `finish` to crunch the
///     // stats and print a summary.
///     benches.finish();
///
///     // You can do other stuff afterward if you want.
///     eprintln!("Done!");
/// }
/// ```
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
	pub fn push(&mut self, mut b: Bench) {
		if ! b.is_spacer() && self.has_name(&b.name) {
			b.stats.replace(Err(BrunchError::DupeName));
		}

		self.0.push(b);
	}

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
			eprintln!(
				"\x1b[1;91mError:\x1b[0m {}",
				BrunchError::NoBench
			);
			return;
		}

		// Build the summaries.
		let mut history = History::default();
		let mut summary = Table::default();
		let names: Vec<Vec<char>> = self.0.iter()
			.filter_map(|b|
				if b.is_spacer() { None }
				else { Some(b.name.chars().collect()) }
			)
			.collect();
		for b in &self.0 {
			summary.push(b, &names, &history);
		}

		// Update the history.
		self.finish_history(&mut history);

		eprintln!("{summary}");
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

impl Benches {
	/// # Has Name.
	fn has_name(&self, name: &str) -> bool {
		self.0.iter().any(|b| b.name == name)
	}
}



#[derive(Debug)]
/// # Benchmark.
///
/// This struct holds a single "bench" you wish to run. See the main crate
/// documentation for more information.
pub struct Bench {
	/// # Benchmark Name.
	name: String,

	/// # Sample Limit.
	samples: NonZeroU32,

	/// # Timeout Limit.
	timeout: Duration,

	/// # Collected Stats.
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

		// Compact and normalize whitespace, but otherwise pass whatever the
		// name is on through.
		let mut ws = false;
		let name: String = name.chars()
			.filter_map(|c|
				if c.is_whitespace() {
					if ws { None }
					else {
						ws = true;
						Some(' ')
					}
				}
				else {
					ws = false;
					Some(c)
				}
			)
			.collect();

		assert!(name.len() <= 65535, "Names cannot be longer than 65,535.");

		Self {
			name,
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
	const fn is_spacer(&self) -> bool { self.name.is_empty() }

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

	#[expect(clippy::missing_panics_doc, reason = "Value is checked.")]
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
	pub const fn with_samples(mut self, samples: u32) -> Self {
		if samples < MIN_SAMPLES.get() { self.samples = MIN_SAMPLES; }
		else {
			// The compiler should optimize this out. MIN_SAMPLES is non-zero
			// so samples must be too.
			self.samples = NonZeroU32::new(samples).unwrap();
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

		let mut times: Vec<Duration> = Vec::with_capacity(usize::saturating_from(self.samples.get()));
		let now = Instant::now();

		for _ in 0..self.samples.get() {
			let now2 = Instant::now();
			let _res = black_box(cb());
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

		let mut times: Vec<Duration> = Vec::with_capacity(usize::saturating_from(self.samples.get()));
		let now = Instant::now();

		for _ in 0..self.samples.get() {
			let seed2 = seed.clone();
			let now2 = Instant::now();
			let _res = black_box(cb(seed2));
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

		let mut times: Vec<Duration> = Vec::with_capacity(usize::saturating_from(self.samples.get()));
		let now = Instant::now();

		for _ in 0..self.samples.get() {
			let seed2 = seed();
			let now2 = Instant::now();
			let _res = black_box(cb(seed2));
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
				"\x1b[1;95mMethod".to_owned(),
				"Mean".to_owned(),
				"Samples\x1b[0m".to_owned(),
				"\x1b[1;95mChange\x1b[0m".to_owned(),
			),
			TableRow::Spacer,
		])
	}
}

impl fmt::Display for Table {
	#[expect(clippy::many_single_char_names, reason = "Consistency is preferred.")]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Maximum column widths.
		let (w1, w2, w3, mut w4) = self.lens();
		let changes = self.show_changes();
		let width =
			if changes { w1 + w2 + w3 + w4 + 12 }
			else {
				w4 = 0;
				w1 + w2 + w3 + 8
			};

		// Pre-generate padding as we'll be slicing lots of things to fit.
		let pad_len = w1.max(w2).max(w3).max(w4);
		let mut pad = String::with_capacity(pad_len);
		for _ in 0..pad_len { pad.push(' '); }

		// Pre-generate the spacer too.
		let mut spacer = String::with_capacity(10 + width);
		spacer.push_str("\x1b[35m");
		for _ in 0..width { spacer.push('-'); }
		spacer.push_str("\x1b[0m\n");

		// Print each line!
		for v in &self.0 {
			let (c1, c2, c3, c4) = v.lens();
			match v {
				TableRow::Normal(a, b, c, d) if changes => writeln!(
					f, "{}{}    {}{}    {}{}    {}{}",
					a, &pad[..w1 - c1],
					&pad[..w2 - c2], b,
					&pad[..w3 - c3], c,
					&pad[..w4 - c4], d,
				)?,
				TableRow::Normal(a, b, c, _) => writeln!(
					f, "{}{}    {}{}    {}{}",
					a, &pad[..w1 - c1],
					&pad[..w2 - c2], b,
					&pad[..w3 - c3], c,
				)?,
				TableRow::Error(a, b) => writeln!(
					f,
					"{}{}    \x1b[38;5;208m{}\x1b[0m",
					a,
					&pad[..w1 - c1],
					b,
				)?,
				TableRow::Spacer => f.write_str(&spacer)?,
			}
		}

		Ok(())
	}
}

impl Table {
	/// # Add Row.
	fn push(&mut self, src: &Bench, names: &[Vec<char>], history: &History) {
		if src.is_spacer() { self.0.push(TableRow::Spacer); }
		else {
			let name = format_name(src.name.chars().collect(), names);
			match src.stats.unwrap_or(Err(BrunchError::NoRun)) {
				Ok(s) => {
					let time = s.nice_mean();
					let diff = history.get(&src.name)
						.and_then(|h| s.is_deviant(h))
						.unwrap_or_else(|| NO_CHANGE.to_owned());
					let (valid, total) = s.samples();
					let samples = format!(
						"\x1b[2m{}\x1b[0;35m/\x1b[0;2m{}\x1b[0m",
						NiceU32::from(valid),
						NiceU32::from(total),
					);

					self.0.push(TableRow::Normal(name, time, samples, diff));
				},
				Err(e) => {
					self.0.push(TableRow::Error(name, e));
				}
			}
		}
	}

	/// # Has Changes?
	///
	/// Returns true if any of the Change columns have a value.
	fn show_changes(&self) -> bool {
		self.0.iter().skip(2).any(|v|
			if let TableRow::Normal(_, _, _, c) = v { c != NO_CHANGE }
			else { false }
		)
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
	/// # Normal Row.
	Normal(String, String, String, String),

	/// # An Error.
	Error(String, BrunchError),

	/// # A Spacer.
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



/// # Format Name.
///
/// Style up a benchmark name by dimming common portions, and highlighting
/// unique ones.
///
/// This approach won't scale well, but the bench count for any given set
/// should be relatively low.
fn format_name(mut name: Vec<char>, names: &[Vec<char>]) -> String {
	let len = name.len();

	// Find the first unique char occurrence.
	let mut pos: usize = names.iter()
		.filter_map(|other|
			if name.eq(other) { None }
			else {
				name.iter()
					.zip(other.iter())
					.position(|(l, r)| l != r)
					.or_else(|| Some(len.min(other.len())))
			}
		)
		.max()
		.unwrap_or_default();

	if 0 < pos && pos < len && ! matches!(name[pos], ':' | '(') {
		// Let's rewind the marker to the position before the last : or (.
		if let Some(pos2) = name[..pos].iter().rposition(|c| matches!(c, ':' | '(')) {
			pos = name[..pos2].iter()
				.rposition(|c| ! matches!(c, ':' | '('))
				.map_or(0, |p| p + 1);
		}
		// Before the last _ or space?
		else if let Some(pos2) = name[..pos].iter().rposition(|c| matches!(c, '_' | ' ')) {
			pos = name[..pos2].iter()
				.rposition(|c| ! matches!(c, '_' | ' '))
				.map_or(0, |p| p + 1);
		}
		// Remove the marker entirely.
		else { pos = 0; }
	}

	if pos == 0 {
		"\x1b[94m".chars()
			.chain(name)
			.chain("\x1b[0m".chars())
			.collect()
	}
	else if pos == len {
		"\x1b[34m".chars()
			.chain(name)
			.chain("\x1b[0m".chars())
			.collect()
	}
	else {
		let b = name.split_off(pos);
		"\x1b[34m".chars()
			.chain(name)
			.chain("\x1b[94m".chars())
			.chain(b)
			.chain("\x1b[0m".chars())
			.collect()
	}
}
