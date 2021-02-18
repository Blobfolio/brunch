/*!
# Brunch: Macros
*/

#[macro_export(local_inner_macros)]
/// # Helper: Benchmarks
///
/// This will generate a `main()` function, bootstrap, and run all supplied
/// benches. Results will be saved and printed afterward nice and neat.
///
/// ## Examples
///
/// ```no_run
/// use brunch::{Bench, benches};
///
/// benches!(
///     Bench::new("some_class", "some_method(x)")
///         .with(callback),
///     Bench::new("other_class", "other_method(x)")
///         .with(callback)
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
