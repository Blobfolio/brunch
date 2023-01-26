/*!
# Brunch: History
*/

use crate::Stats;
use std::{
	collections::BTreeMap,
	ffi::OsStr,
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
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



/// # Deserialization.
trait Deserialize<'a>: Sized {
	/// # Deserialize.
	///
	/// This deserializes `Self` from some number of leading bytes, returning
	/// it along with the rest of the slice.
	fn deserialize(raw: &'a [u8]) -> Option<(Self, &'a [u8])>;
}

macro_rules! deserialize {
	($($size:literal $ty:ty),+) => ($(
		impl Deserialize<'_> for $ty {
			fn deserialize(raw: &[u8]) -> Option<(Self, &[u8])> {
				let (bytes, raw) = split_array::<$size>(raw)?;
				Some((Self::from_be_bytes(bytes), raw))
			}
		}
	)+);
}

deserialize!(2 u16, 4 u32, 8 f64);

impl<'a> Deserialize<'a> for &'a str {
	fn deserialize(raw: &'a [u8]) -> Option<(Self, &'a [u8])> {
		let (len, raw) = u16::deserialize(raw)?;
		let len = usize::from(len);
		if raw.len() < len { None }
		else {
			let (lbl, raw) = raw.split_at(len);
			let lbl = std::str::from_utf8(lbl).ok()?.trim();
			Some((lbl, raw))
		}
	}
}

impl Deserialize<'_> for Stats {
	fn deserialize(raw: &[u8]) -> Option<(Self, &[u8])> {
		let (total, raw) = u32::deserialize(raw)?;
		let (valid, raw) = u32::deserialize(raw)?;
		let (deviation, raw) = f64::deserialize(raw)?;
		let (mean, raw) = f64::deserialize(raw)?;

		let out = Self { total, valid, deviation, mean };
		Some((out, raw))
	}
}



/// # Deserialize.
///
/// This deserializes the stored history data, if any. This will happily return
/// an empty map if no benchmarks are present, but will return `None` if there
/// are any structural issues, like a magic mismatch or invalid chunk lengths.
///
/// See `serialize` for more details about the format.
fn deserialize(raw: &[u8]) -> Option<HistoryData> {
	let mut raw = raw.strip_prefix(MAGIC)?;
	let mut out = HistoryData::default();

	while ! raw.is_empty() {
		let (lbl, rest) = <&str>::deserialize(raw)?;
		let (stats, rest) = Stats::deserialize(rest)?;

		// Push the result if it's valid.
		if ! lbl.is_empty() && stats.is_valid() {
			out.insert(lbl.to_owned(), stats);
		}

		// Update the slice for the next go-round.
		raw = rest;
	}

	Some(out)
}

#[allow(clippy::option_if_let_else)]
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
	deserialize(&raw)
}

/// # Serialize.
///
/// This cheaply serializes the run-to-run history data to a simple, compact
/// binary structure, more or less placing all the fields back-to-back.
///
/// The output begins with an 8-byte ASCII string, comprising `BRUNCH` and a
/// format version (in case we ever need to alter the structure).
///
/// After that, zero or more entries follow, each with the following format:
///
/// | Length | Format | Data |
/// | ------ | ------ | ---- |
/// | 2 | `u16` | Length of bench label. |
/// | _n_ | UTF-8 | Bench label. |
/// | 4 | `u32` | Total samples. |
/// | 4 | `u32` | Valid samples. |
/// | 8 | `f64` | Standard deviation. |
/// | 8 | `f64` | Average time. |
///
/// All number sequences use the Big Endian layout.
fn serialize(history: &HistoryData) -> Vec<u8> {
	// Start with the magic header.
	let mut out = Vec::with_capacity(64 * history.len());
	out.extend_from_slice(MAGIC);

	// Write each benchmark entry.
	for (lbl, s) in history.iter() {
		// We panic on long names so this should never fail, but just in case,
		// let's check.
		if let Ok(len) = u16::try_from(lbl.len()) {
			// Entries begin with the length of the label, then the label itself.
			out.extend_from_slice(&len.to_be_bytes());
			out.extend_from_slice(lbl.as_bytes());

			// Total, valid, deviation, and mean follow, in that order.
			out.extend_from_slice(&s.total.to_be_bytes());
			out.extend_from_slice(&s.valid.to_be_bytes());
			out.extend_from_slice(&s.deviation.to_be_bytes());
			out.extend_from_slice(&s.mean.to_be_bytes());
		}
	}

	out
}

#[allow(unsafe_code)]
/// # Split Array.
///
/// This is basically a rewrite of the nightly-only `slice::split_array_ref`
/// method, except instead of panicking it will return `None` if the length too
/// small to split.
fn split_array<const S: usize>(raw: &[u8]) -> Option<([u8; S], &[u8])> {
	if S <= raw.len() {
		// Safety: we know there are at least S bytes.
		Some(unsafe {(
			*(raw.get_unchecked(..S).as_ptr().cast()),
			raw.get_unchecked(S..),
		)})
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
	use dactyl::total_cmp;

	#[test]
	fn t_serialize() {
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
		let mut h = ENTRIES.into_iter().map(|(k, v)| (k.to_owned(), v)).collect::<HistoryData>();

		// Serialize it.
		let s = serialize(&h);
		assert!(s.starts_with(MAGIC), "Missing magic header.");

		// Deserialize it.
		let d = deserialize(&s).expect("Deserialization failed.");

		// The deserialized length should match our reference length.
		assert_eq!(h.len(), d.len(), "Deserialized length mismatch.");

		// Make sure the entries are unchanged.
		for (lbl, stat) in ENTRIES {
			let tmp = d.get(lbl).expect("Missing entry!");
			assert_eq!(stat.total, tmp.total, "Total changed.");
			assert_eq!(stat.valid, tmp.valid, "Valid changed.");
			assert!(total_cmp!((stat.deviation) == (tmp.deviation)), "Deviation changed.");
			assert!(total_cmp!((stat.mean) == (tmp.mean)), "Mean changed.");
		}

		// Let's add a logically-suspect entry to the history, and make sure
		// it gets stripped out during deserialize.
		h.insert("A Suspect One".to_owned(), Stats {
			total: 200,
			valid: 300,
			deviation: 0.000400123,
			mean: 0.0000122,
		});
		h.insert(String::new(), Stats {
			total: 500,
			valid: 300,
			deviation: 0.000400123,
			mean: 0.0000122,
		});

		// Make sure these exist in the reference struct.
		assert!(h.get("A Suspect One").is_some());
		assert!(h.get("").is_some());

		// Another round of in/out.
		let mut s = serialize(&h);
		let d = deserialize(&s).expect("Deserialization failed.");

		// Check they got filtered out during deserialization.
		assert_eq!(ENTRIES.len(), d.len(), "Deserialized length mismatch.");
		assert!(d.get("A Suspect One").is_none()); // Shouldn't be here.
		assert!(d.get("").is_none());

		// To be extra safe, let's recheck the valid entries to make sure they
		// didn't get screwed up in any way.
		for (lbl, stat) in ENTRIES {
			let tmp = d.get(lbl).expect("Missing entry!");
			assert_eq!(stat.total, tmp.total, "Total changed.");
			assert_eq!(stat.valid, tmp.valid, "Valid changed.");
			assert!(total_cmp!((stat.deviation) == (tmp.deviation)), "Deviation changed.");
			assert!(total_cmp!((stat.mean) == (tmp.mean)), "Mean changed.");
		}

		// Make sure deserializing doesn't do anything on bad data.
		s.pop().unwrap();
		assert!(deserialize(&s).is_none());
		assert!(deserialize(&[]).is_none());
	}
}
