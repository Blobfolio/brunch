/*!
# Brunch: History
*/

use crate::Stats;
use serde::{
	Serialize,
	Deserialize,
};
use std::{
	collections::HashMap,
	fs::File,
	io::BufReader,
	path::PathBuf,
};



#[doc(hidden)]
#[derive(Debug, Serialize, Deserialize, Clone)]
/// # History.
///
/// This is triggered automatically when using the [`benches`] macro; it is
/// not intended to be called manually.
pub struct History(HashMap<String, Stats>);

impl Default for History {
	fn default() -> Self {
		File::open(Self::path())
			.ok()
			.and_then(|file| {
				let r = BufReader::new(file);
				serde_json::from_reader(r).ok()
			})
			.unwrap_or_else(|| Self(HashMap::new()))
	}
}

impl History {
	/// # Insert/Update Record.
	///
	/// Record the current stats, and return the old ones, if any.
	pub fn insert(&mut self, key: String, stats: Stats) -> Option<Stats> {
		self.0.insert(key, stats)
	}

	/// # Path.
	///
	/// This path is used to temporarily store historical results.
	fn path() -> PathBuf {
		let mut p = std::env::temp_dir();
		p.push("_brunch.json");
		p
	}

	/// # Save.
	///
	/// The last run for any given test is saved to a temporary file. These
	/// values, if any, will be used to show the relative difference if the
	/// test is re-run.
	pub fn save(&self) {
		if let Ok(mut out) = File::create(Self::path()) {
			let _res = serde_json::to_writer(&mut out, &self);
		}
	}
}
