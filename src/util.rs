/*!
# Brunch: Utility Functions
*/

use unicode_width::UnicodeWidthChar;



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
