/*!
# Brunch: Utility Functions
*/

use unicode_width::UnicodeWidthChar;



#[cfg(not(no_brunch_black_box))]
#[allow(unsafe_code)]
#[doc(hidden)]
#[deprecated(since = "0.3.7", note = "update to Rust 1.66, which stabilized black_box.")]
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
