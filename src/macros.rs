/*!
# Brunch: Macros
*/

#[macro_export(local_inner_macros)]
/// # Helper: Benchmarks
///
/// The [`benches`] macro is the easiest way to run `Brunch` benchmarks.
///
/// Simply pass a comma-separated list of all the [`Bench`](crate::Bench)
/// objects you want to run, and it will handle the setup, running, tabulation,
/// and give you a nice summary at the end.
///
/// By default, this macro will generate the `main()` entrypoint too, but you
/// can suppress this by adding "inline:" as the first argument.
///
/// ## Examples
///
/// The default usage would look something like the following:
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
///     Bench::new("usize::checked_add(2)")
///         .run(callback),
///     Bench::new("usize::checked_mul(2)")
///         .run(callback2)
/// );
/// ```
///
/// When declaring your own main entrypoint, you need to add "inline:" as the
/// first argument. The list of [`Bench`](crate::Bench) instances follow as
/// usual after that.
///
/// ```no_run
/// use brunch::{Bench, benches};
///
/// /// # Custom Main.
/// fn main() {
///     // A typical use case for the "inline" variant would be to declare
///     // an owned variable for a benchmark that needs to return a reference
///     // (to e.g. keep Rust from complaining about lifetimes).
///     let v = vec![0_u8, 1, 2, 3, 4, 5];
///
///     // The macro call goes here!
///     benches!(
///         inline:
///
///         Bench::new("vec::as_slice()").run(|| v.as_slice()),
///     );
///
///     // You can also do other stuff afterwards if you want.
///     eprintln!("Done!");
/// }
/// ```
///
/// For even more control over the flow, skip the macro and just use [`Benches`](crate::Benches)
/// directly.
macro_rules! benches {
	(inline: $($benches:expr),+ $(,)?) => {{
		let mut benches = $crate::Benches::default();
		$(
			benches.push($benches);
		)+
		benches.finish();
	}};

	($($benches:expr),+ $(,)?) => {
		/// # Benchmarks!
		fn main() {
			// Announce that we've started.
			::std::eprint!("\x1b[1;38;5;199mStarting:\x1b[0m Running benchmark(s). Stand by!\n\n");

			// Run the benches.
			let mut benches = $crate::Benches::default();
			$(
				// Print a dot to show some progress.
				::std::eprint!("\x1b[1;34m•\x1b[0m");

				benches.push($benches);
			)+

			// Give some space.
			::std::eprint!("\n\n");

			// Finish!
			benches.finish();
		}
	};
}
