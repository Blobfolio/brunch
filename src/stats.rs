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
};
use serde::{
	de,
	Deserialize,
	Serialize,
	ser::{
		self,
		SerializeStruct,
	},
};
use std::{
	cmp::Ordering,
	collections::BTreeMap,
	fmt,
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
	time::Duration,
};



#[doc(hidden)]
#[derive(Debug, Clone)]
/// # History.
///
/// This is triggered automatically when using the [`benches`] macro; it is
/// not intended to be called manually.
pub(crate) struct History(BTreeMap<String, Stats>);

impl Default for History {
	fn default() -> Self {
		load_history().unwrap_or_else(|| Self(BTreeMap::default()))
	}
}

impl<'de> Deserialize<'de> for History {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: de::Deserializer<'de> {
		let mut out: BTreeMap<String, Stats> = de::Deserialize::deserialize(deserializer)?;
		// Silently strip out nonsense.
		out.retain(|k, v| ! k.is_empty() && v.is_valid());
		Ok(Self(out))
	}
}

impl Serialize for History {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: ser::Serializer { self.0.serialize(serializer) }
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
			if let Ok(out) = serde_json::to_vec(&self) {
				let _res = f.write_all(&out).and_then(|_| f.flush());
			}
		}
	}
}



#[derive(Debug, Clone, Copy)]
/// # Runtime Stats!
pub(crate) struct Stats {
	/// # Total Samples.
	total: usize,

	/// # Valid Samples.
	valid: usize,

	/// # Standard Deviation.
	deviation: f64,

	/// # Mean Duration of Valid Samples.
	mean: f64,
}

impl<'de> Deserialize<'de> for Stats {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: de::Deserializer<'de> {
		enum Field { Total, Valid, Deviation, Mean }

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where D: de::Deserializer<'de> {
				struct FieldVisitor;

				impl<'de> de::Visitor<'de> for FieldVisitor {
					type Value = Field;

					fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
						f.write_str("`total`, `valid`, `deviation`, or `mean`")
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where
						E: de::Error,
					{
						match value {
							"total" => Ok(Field::Total),
							"valid" => Ok(Field::Valid),
							"deviation" => Ok(Field::Deviation),
							"mean" => Ok(Field::Mean),
							_ => Err(de::Error::unknown_field(value, FIELDS)),
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct StatsVisitor;

		impl<'de> de::Visitor<'de> for StatsVisitor {
			type Value = Stats;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct Stats")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<Stats, V::Error>
			where V: de::SeqAccess<'de> {
				let total: usize = seq.next_element()?
					.ok_or_else(|| de::Error::invalid_length(0, &self))?;
				let valid: usize = seq.next_element()?
					.ok_or_else(|| de::Error::invalid_length(1, &self))?;
				let deviation: f64 = seq.next_element()?
					.ok_or_else(|| de::Error::invalid_length(2, &self))?;
				let mean: f64 = seq.next_element()?
					.ok_or_else(|| de::Error::invalid_length(3, &self))?;

				Ok(Stats{ total, valid, deviation, mean })
			}

			fn visit_map<V>(self, mut map: V) -> Result<Stats, V::Error>
			where V: de::MapAccess<'de> {
				let mut total: Option<usize> = None;
				let mut valid: Option<usize> = None;
				let mut deviation: Option<f64> = None;
				let mut mean: Option<f64> = None;

				while let Some(key) = map.next_key()? {
					match key {
						Field::Total => {
							if total.is_some() {
								return Err(de::Error::duplicate_field("total"));
							}
							total.replace(map.next_value()?);
						},
						Field::Valid => {
							if valid.is_some() {
								return Err(de::Error::duplicate_field("valid"));
							}
							valid.replace(map.next_value()?);
						},
						Field::Deviation => {
							if deviation.is_some() {
								return Err(de::Error::duplicate_field("deviation"));
							}
							deviation.replace(map.next_value()?);
						},
						Field::Mean => {
							if mean.is_some() {
								return Err(de::Error::duplicate_field("mean"));
							}
							mean.replace(map.next_value()?);
						},
					}
				}

				let total = total.ok_or_else(|| de::Error::missing_field("total"))?;
				let valid = valid.ok_or_else(|| de::Error::missing_field("valid"))?;
				let deviation = deviation.ok_or_else(|| de::Error::missing_field("deviation"))?;
				let mean = mean.ok_or_else(|| de::Error::missing_field("mean"))?;
				Ok(Stats{ total, valid, deviation, mean })
			}
		}

		const FIELDS: &[&str] = &["total", "valid", "deviation", "mean"];
		deserializer.deserialize_struct("Stats", FIELDS, StatsVisitor)
	}
}

impl Serialize for Stats {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: ser::Serializer {
		let mut state = serializer.serialize_struct("Stats", 4)?;

		state.serialize_field("total", &self.total)?;
		state.serialize_field("valid", &self.valid)?;
		state.serialize_field("deviation", &self.deviation)?;
		state.serialize_field("mean", &self.mean)?;

		state.end()
	}
}

impl TryFrom<Vec<Duration>> for Stats {
	type Error = BrunchError;
	fn try_from(samples: Vec<Duration>) -> Result<Self, Self::Error> {
		let total = samples.len();
		if total < MIN_SAMPLES {
			return Err(BrunchError::TooSmall(total));
		}

		// Crunch!
		let mut calc = Abacus::from(samples);
		calc.prune_outliers();

		let valid = calc.len();
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
	pub(crate) const fn samples(self) -> (usize, usize) { (self.valid, self.total) }

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



/// # History Path.
///
/// Return the file path history should be written to or read from.
fn history_path() -> Option<PathBuf> {
	if std::env::var("NO_BRUNCH_HISTORY").map_or(false, |s| s.trim() == "1") { None }
	else {
		let mut p = try_dir(std::env::var_os("BRUNCH_DIR"))
			.or_else(|| try_dir(Some(std::env::temp_dir())))?;
		p.push("__brunch.json");
		Some(p)
	}
}

/// # Read History.
///
/// Load and return the history, if any.
fn load_history() -> Option<History> {
	let file = history_path()?;
	let raw = std::fs::read(file).ok()?;
	serde_json::from_slice(&raw).ok()
}

/// # Try Dir.
///
/// Test if the thing is a directory and return it.
fn try_dir<P: AsRef<Path>>(dir: Option<P>) -> Option<PathBuf> {
	let dir = std::fs::canonicalize(dir?).ok()?;
	if dir.is_dir() { Some(dir) }
	else { None }
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_stats_serde() {
		let stat = Stats {
			total: 2500,
			valid: 2496,
			deviation: 0.000000123,
			mean: 0.0000022,
		};

		// Serialize and deserialize.
		let s = serde_json::to_string(&stat).expect("Serialization failed.");
		let d: Stats = serde_json::from_str(&s).expect("Deserialization failed.");

		// Make sure we end up where we began.
		assert_eq!(stat.total, d.total, "Deserialization changed total.");
		assert_eq!(stat.valid, d.valid, "Deserialization changed valid.");
		assert!(
			total_cmp!((stat.deviation) == (d.deviation)),
			"Deserialization changed deviation."
		);
		assert!(
			total_cmp!((stat.mean) == (d.mean)),
			"Deserialization changed mean."
		);
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
