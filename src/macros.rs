/*!
# Brunch: Macros
*/

#[macro_export(local_inner_macros)]
/// # Helper: Benchmarks
///
/// This will generate a `main()` function, bootstrap, and run all supplied
/// benches. Results will be saved and printed afterward nice and neat.
///
/// See the main crate documentation for more information.
///
/// ## Examples
///
/// ```no_run
/// use brunch::{Bench, benches};
///
/// // Example benchmark adding 2+2.
/// fn callback() -> Option<usize> { 2_usize.checked_add(2) }
///
/// // Example benchmark multiplying 2x2.
/// fn callback2() -> Option<usize> { 2_usize.checked_mul(2) }
///
/// benches!(
///     Bench::new("usize", "checked_add(2)")
///         .with(callback),
///     Bench::new("usize", "checked_mul(2)")
///         .with(callback2)
/// );
/// ```
macro_rules! benches {
	($($benches:expr),+) => {
		/// # Benchmarks!
		fn main() {
			// This can take a while; give 'em a message of hope.
			::std::print!("\x1b[1;38;5;199mStarting:\x1b[0m Running benchmark(s). Stand by!\n\n");

			// Run the benches.
			let mut benches: Vec<$crate::Bench> = Vec::new();
			$(
				benches.push($benches);
			)+

			$crate::analyze(&mut benches);
		}
	};
}
