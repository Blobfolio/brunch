/*!
# Brunch: Utility Functions
*/

use dactyl::NiceU32;
use num_traits::FromPrimitive;
use unicode_width::UnicodeWidthChar;



#[allow(unsafe_code)]
#[doc(hidden)]
/// # Black Box.
///
/// This pseudo-black box is stolen from [`easybench`](https://crates.io/crates/easybench), which
/// stole it from `Bencher`.
///
/// The gist is it mostly works, but may fail to prevent the compiler from
/// optimizing it away in some cases. Avoiding nightly, it is the best we've
/// got.
pub(crate) fn black_box<T>(dummy: T) -> T {
	unsafe {
		let ret = std::ptr::read_volatile(&dummy);
		std::mem::forget(dummy);
		ret
	}
}

/// # Format w/ Unit.
///
/// Give us a nice comma-separated integer with two decimal places and an
/// appropriate unit (running from pico seconds to milliseconds).
pub(crate) fn format_time(mut time: f64) -> String {
	let unit: &str =
		if time < 0.000_001 {
			time *= 1_000_000_000.000;
			"ns"
		}
		else if time < 0.001 {
			time *= 1_000_000.000;
			"\u{3bc}s"
		}
		else if time < 1.0 {
			time *= 1_000.000;
			"ms"
		}
		else if time < 60.0 {
			"s "
		}
		else {
			time /= 60.0;
			"m "
		};

	format!(
		"\x1b[1m{}.{:02} {}\x1b[0m",
		NiceU32::from(u32::from_f64(time.trunc()).unwrap_or_default()).as_str(),
		u32::from_f64(f64::floor(time.fract() * 100.0)).unwrap_or_default(),
		unit
	)
}

/// # Width.
///
/// Return the printable width of a string. This is somewhat naive, but gets
/// closer than merely calling `String::len`.
pub(crate) fn width(src: &str) -> usize {
	let mut in_ansi: bool = false;
	src.chars()
		.fold(0_usize, |w, c| {
			// In ANSI.
			if in_ansi {
				if matches!(c, 'm' | 'A' | 'K') { in_ansi = false; }
				w
			}
			// New ANSI.
			else if c == '\x1b' {
				in_ansi = true;
				w
			}
			// Something else.
			else {
				UnicodeWidthChar::width(c).map_or(w, |w2| w2 + w)
			}
		})
}
