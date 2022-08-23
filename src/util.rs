/*!
# Brunch: Utility Functions
*/

use std::cmp::Ordering;
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

/// # Float < Float.
pub(crate) fn float_lt(a: f64, b: f64) -> bool {
	matches!(a.total_cmp(&b), Ordering::Less)
}

/// # Float <= Float.
pub(crate) fn float_le(a: f64, b: f64) -> bool {
	matches!(a.total_cmp(&b), Ordering::Less | Ordering::Equal)
}

/// # Float == Float.
pub(crate) fn float_eq(a: f64, b: f64) -> bool {
	matches!(a.total_cmp(&b), Ordering::Equal)
}

/// # Float >= Float.
pub(crate) fn float_ge(a: f64, b: f64) -> bool {
	matches!(a.total_cmp(&b), Ordering::Equal | Ordering::Greater)
}

/// # Float > Float.
pub(crate) fn float_gt(a: f64, b: f64) -> bool {
	matches!(a.total_cmp(&b), Ordering::Greater)
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cmp_float() {
		assert!(float_lt(5.0, 10.0));
		assert!(! float_lt(10.0, 10.0));
		assert!(! float_lt(11.0, 10.0));

		assert!(float_le(5.0, 10.0));
		assert!(float_le(10.0, 10.0));
		assert!(! float_le(11.0, 10.0));

		assert!(float_eq(5.0, 5.0));
		assert!(! float_eq(5.0, 5.00000001));

		assert!(float_ge(15.0, 10.0));
		assert!(float_ge(10.0, 10.0));
		assert!(! float_ge(9.999, 10.0));

		assert!(float_gt(15.0, 10.0));
		assert!(! float_gt(10.0, 10.0));
		assert!(! float_gt(9.999, 10.0));
	}
}
