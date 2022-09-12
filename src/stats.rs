/*!
# Brunch: Stats
*/

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
	collections::BTreeMap,
	ffi::OsStr,
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
	sync::Once,
	time::Duration,
};



/// # History Inner Data.
type HistoryData = BTreeMap<String, Stats>;

/// # History Default File Name.
const HISTORY_FILE: &str = "__brunch.last";

/// # History Magic Header.
///
/// This provides a quick way to know whether or not a given file might be a
/// `Brunch` history. The trailing digits act like a format version; they'll
/// get bumped any time the data format changes, to prevent compatibility
/// issues between releases.
const MAGIC: &[u8] = b"BRUNCH00";

/// # Warn once about use of `BRUNCH_DIR` env.
static BRUNCH_DIR_ENV: Once = Once::new();



#[doc(hidden)]
#[derive(Debug, Clone)]
/// # History.
///
/// This is triggered automatically when using the [`benches`] macro; it is
/// not intended to be called manually.
pub(crate) struct History(HistoryData);

impl Default for History {
	fn default() -> Self {
		Self(load_history().unwrap_or_default())
	}
}

impl History {
	/// # Get Entry.
	pub(crate) fn get(&self, key: &str) -> Option<Stats> {
		self.0.get(key).copied()
	}

	/// # Insert.
	pub(crate) fn insert(&mut self, key: &str, v: Stats) {
		self.0.insert(key.to_owned(), v);
	}

	/// # Save.
	pub(crate) fn save(&self) {
		if let Some(mut f) = history_path().and_then(|f| File::create(f).ok()) {
			let out = serialize(&self.0);
			let _res = f.write_all(&out).and_then(|_| f.flush());
		}
	}
}



#[derive(Debug, Clone, Copy)]
/// # Runtime Stats!
pub(crate) struct Stats {
	/// # Total Samples.
	total: u32,

	/// # Valid Samples.
	valid: u32,

	/// # Standard Deviation.
	deviation: f64,

	/// # Mean Duration of Valid Samples.
	mean: f64,
}

impl TryFrom<Vec<Duration>> for Stats {
	type Error = BrunchError;
	fn try_from(samples: Vec<Duration>) -> Result<Self, Self::Error> {
		let total = u32::saturating_from(samples.len());
		if total < MIN_SAMPLES {
			return Err(BrunchError::TooSmall(total));
		}

		// Crunch!
		let mut calc = Abacus::from(samples);
		calc.prune_outliers();

		let valid = u32::saturating_from(calc.len());
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

		format!("\x1b[0;1m{} {}\x1b[0m", NiceFloat::from(mean).precise_str(2), unit)
	}

	/// # Samples.
	///
	/// Return the valid/total samples.
	pub(crate) const fn samples(self) -> (u32, u32) { (self.valid, self.total) }

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



/// # Deserialize.
///
/// This deserializes the inner data for a `History` object from our custom
/// format. See `serialize` for more details.
///
/// This won't fail, but will strip out invalid entries as it comes across
/// them.
///
/// Any time we change the version portion of our `MAGIC` constant, results
/// from older versions will refuse to parse, resulting in an empty set.
fn deserialize(raw: &[u8]) -> HistoryData {
	let mut out = HistoryData::default();

	// It should start with our magic header.
	let mut raw = match raw.strip_prefix(MAGIC) {
		Some(r) => r,
		None => return out,
	};

	while let Some((lbl, stats, rem)) = deserialize_entry(raw) {
		// Keep it?
		if ! lbl.is_empty() && stats.is_valid() {
			out.insert(lbl.to_owned(), stats);
		}

		// Are we done?
		if rem.is_empty() { break; }
		raw = rem;
	}

	out
}

/// # Deserialize Stat.
///
/// This deserializes a single benchmark entry (a label and `Stats`), returning
/// the pieces along with the remainder of the input slice if valid.
///
/// This doesn't worry about key/value sanity, so will only abort if the
/// lengths are wrong or the label cannot be stringified.
fn deserialize_entry(raw: &[u8]) -> Option<(&str, Stats, &[u8])> {
	const STAT_SIZE: usize = 4 + 4 + 8 + 8;

	// Find the length of the label.
	let (len, raw) = split_array::<2>(raw)?;
	let len = u16::from_be_bytes(len) as usize;
	if raw.len() < len + STAT_SIZE { return None; }

	// Parse the label.
	let (lbl, raw) = raw.split_at(len);
	let lbl = std::str::from_utf8(lbl).ok()?.trim();

	// Parse total, valid, deviation, and mean.
	let (total, raw) = split_array::<4>(raw)?;
	let total = u32::from_be_bytes(total);

	let (valid, raw) = split_array::<4>(raw)?;
	let valid = u32::from_be_bytes(valid);

	let (deviation, raw) = split_array::<8>(raw)?;
	let deviation = f64::from_be_bytes(deviation);

	let (mean, raw) = split_array::<8>(raw)?;
	let mean = f64::from_be_bytes(mean);

	Some((lbl, Stats { total, valid, deviation, mean }, raw))
}

/// # History Path.
///
/// Return the file path history should be written to or read from.
fn history_path() -> Option<PathBuf> {
	// No history?
	if std::env::var("NO_BRUNCH_HISTORY").map_or(false, |s| s.trim() == "1") { None }
	// To a specific file?
	else if let Some(p) = std::env::var_os("BRUNCH_HISTORY") {
		let p: &Path = p.as_ref();

		// If the path exists, it cannot be a directory.
		if p.is_dir() { return None; }

		// Tease out the parent.
		let parent = try_dir(p.parent())
			.or_else(|| try_dir(std::env::current_dir().ok()))?;

		// Tease out the file name.
		let name = match p.file_name() {
			Some(n) if ! n.is_empty() => n,
			_ => OsStr::new(HISTORY_FILE),
		};

		Some(parent.join(name))
	}
	// To a specific directory?
	else if let Some(p) = try_dir(std::env::var_os("BRUNCH_DIR")) {
		// Fake a deprecation notice since we can't apply the real one to an
		// env value.
		BRUNCH_DIR_ENV.call_once(|| {
			eprint!("\x1b[1;38;5;3mwarning\x1b[0;1m: use of deprecated env `BRUNCH_DIR`: use `BRUNCH_HISTORY` (with full file path, not directory) instead.\x1b[0m\n\n");
		});

		Some(p.join(HISTORY_FILE))
	}
	// To the default temporary location?
	else {
		let p = try_dir(Some(std::env::temp_dir()))?;
		Some(p.join(HISTORY_FILE))
	}
}

/// # Read History.
///
/// Load and return the history, if any.
fn load_history() -> Option<HistoryData> {
	let file = history_path()?;
	let raw = std::fs::read(file).ok()?;
	Some(deserialize(&raw))
}

/// # Serialize.
///
/// This is a cheap, custom serialization structure for history. It begins with
/// a magic header, then each entry.
///
/// Each entry starts with a u16 corresponding to the length of the bench name,
/// then the name itself. After that, 24 bytes corresponding to the total (u32),
/// valid (u32), deviation (f64), and mean (f64) appear.
///
/// All integers use Big Endian storage.
fn serialize(history: &HistoryData) -> Vec<u8> {
	// Magic header.
	let mut out = MAGIC.to_vec();

	// Write each section.
	for (lbl, s) in history.iter() {
		// We panic on long names so this should never fail, but just in case,
		// let's check.
		let len = match u16::try_from(lbl.len()) {
			Ok(l) => l,
			Err(_) => continue,
		};

		// Write the label.
		out.extend_from_slice(&len.to_be_bytes());
		out.extend_from_slice(lbl.as_bytes());

		// Write total, valid, deviation, and mean.
		out.extend_from_slice(&s.total.to_be_bytes());
		out.extend_from_slice(&s.valid.to_be_bytes());
		out.extend_from_slice(&s.deviation.to_be_bytes());
		out.extend_from_slice(&s.mean.to_be_bytes());
	}

	out
}

/// # Split Array.
///
/// This takes the first `S` bytes and forms an array, then returns the rest
/// as a slice.
fn split_array<const S: usize>(raw: &[u8]) -> Option<([u8; S], &[u8])> {
	if S <= raw.len() {
		let (l, r) = raw.split_at(S);
		let l: [u8; S] = l.try_into().ok()?;
		Some((l, r))
	}
	else { None }
}

/// # Try Dir.
///
/// Test if the thing is a directory and return it.
fn try_dir<P: AsRef<Path>>(dir: Option<P>) -> Option<PathBuf> {
	let dir = dir?;
	let dir: &Path = dir.as_ref();

	// Create the directory if it doesn't exist.
	if ! dir.exists() { std::fs::create_dir_all(dir).ok()?; }

	// Canonicalize it.
	let dir = std::fs::canonicalize(dir).ok()?;

	// Return it so long as it is a directory.
	if dir.is_dir() { Some(dir) }
	else { None }
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_stats_ser() {
		const ENTRIES: [(&str, Stats); 2] = [
			(
				"The First One",
				Stats {
					total: 2500,
					valid: 2496,
					deviation: 0.000000123,
					mean: 0.0000022,
				},
			),
			(
				"The Second One",
				Stats {
					total: 300,
					valid: 222,
					deviation: 0.000400123,
					mean: 0.0000122,
				},
			),
		];

		// Our reference.
		let h = ENTRIES.into_iter().map(|(k, v)| (k.to_owned(), v)).collect::<HistoryData>();

		// Serialize it.
		let s = serialize(&h);

		// Deserialize it.
		let d = deserialize(&s);

		// The deserialized length should match our reference length.
		assert_eq!(h.len(), d.len());

		// Make sure the entries are unchanged.
		for (lbl, stat) in ENTRIES {
			let tmp = d.get(lbl).expect("Missing entry!");
			assert_eq!(stat.total, tmp.total, "Total changed.");
			assert_eq!(stat.valid, tmp.valid, "Valid changed.");
			assert!(total_cmp!((stat.deviation) == (tmp.deviation)), "Deviation changed.");
			assert!(total_cmp!((stat.mean) == (tmp.mean)), "Mean changed.");
		}
	}

	#[test]
	fn t_stats_valid() {
		let mut stat = Stats {
			total: 2500,
			valid: 2496,
			deviation: 0.000000123,
			mean: 0.0000022,
		};

		assert!(stat.is_valid(), "Stat should be valid.");

		stat.total = 100;
		assert!(! stat.is_valid(), "Insufficient total.");

		stat.valid = 100;
		assert!(stat.is_valid(), "Stat should be valid.");

		stat.valid = 30;
		assert!(! stat.is_valid(), "Insufficient samples.");

		stat.valid = 100;
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
