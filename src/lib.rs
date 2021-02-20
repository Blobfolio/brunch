/*!
# Brunch

`Brunch` is a very simple Rust micro-benchmark runner inspired by [`easybench`](https://crates.io/crates/easybench). It has roughly a million times fewer dependencies than [`criterion`](https://crates.io/crates/criterion), does not require nightly, and maintains a "last run" state so can show relative changes benchmark-to-benchmark. The formatting is also quite pretty.

As with all Rust benchmarking, there are a lot of caveats, and results might be artificially fast or slow. For best resuilts, build optimized, avoid heavy setup contexts, and test different bench setups to find the most "honest" representation.

In theory, this library can reach pico-second scales (it clocks increasingly large batches and divides accordingly), but background noise and setup overhead will likely prevent times getting quite as low as they might "actually" be. It can go as long as milliseconds, but might require increased time limits to reach sufficient samples in such cases.



## Work in Progress

This crate is still under heavy development. It is subject to change (and almost certainly going to) so you probably don't want to rely on it in production yet. But that said, feel free to poke around, steal code, find inspiration, etc.



## Installation

Add `brunch` to your `dev-dependencies` in `Cargo.toml`, like:

```
[dev-dependencies]
brunch = "0.1.*"
```

Benchemarks are also defined in `Cargo.toml` the usual way. Just be sure to set `harness = false`:

```
[[bench]]
name = "encode"
harness = false
```



## Usage

Setup is currently simple if primitive, requiring you drop a call to the [`benches`] macro in the benchmark file. It will generate a `main()` method, run the supplied benchmarks, and give you the results.

An example bench file would look something like:

```
use brunch::Bench;
use dactyl::NiceU8;
use std::time::Duration;

brunch::benches!(
    Bench::new("dactyl::NiceU8", "from(0)")
        .timed(Duration::from_secs(1))
        .with(|| NiceU8::from(0_u8)),

    Bench::new("dactyl::NiceU8", "from(18)")
        .timed(Duration::from_secs(1))
        .with(|| NiceU8::from(18_u8)),

    Bench::new("dactyl::NiceU8", "from(101)")
        .timed(Duration::from_secs(1))
        .with(|| NiceU8::from(101_u8)),

    Bench::new("dactyl::NiceU8", "from(u8::MAX)")
        .timed(Duration::from_secs(1))
        .with(|| NiceU8::from(u8::MAX))
);
```

The [`Bench`] struct represents a benchmark. It takes two label arguments intended to represent a shared base (for the included benchmarks) and the unique bit, usually a method/value.

By default, each benchmark will run for approximately three seconds. This can be changed using the chained [`Bench::timed`] method as shown above.

There are currently three styles of callback:

| Method | Signature | Description |
| ------ | --------- | ----------- |
| `with` | `FnMut() -> O` | Execute a self-contained callback. |
| `with_setup` | `FnMut(I) -> O` | Execute a callback seeded with a (cloneable) value. |
| `with_setup_ref` | `FnMut(&I) -> O` | Execute a callback seeded with a referenced value. |

*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::map_err_ignore)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



mod bench;
#[macro_use]
mod macros;

pub use bench::{
	Bench,
	BenchResult,
	error::BenchError,
	history::History,
	stats::Stats,
};



/// # Analyze Results.
///
/// This method is called by the [`benches`] macro. It is not intended to be
/// called directly.
pub fn analyze(benches: &mut Vec<Bench>) {
	// Update histories.
	let mut history = History::default();
	benches.iter_mut().for_each(|x| x.history(&mut history));
	history.save();

	// Pull results.
	let results: Vec<BenchResult> = benches.iter()
		.map(BenchResult::from)
		.collect();

	// Count up the lengths so we can display pretty-like.
	let (c1, c2, c3) = results.iter()
		.fold((0, 0, 0), |(c1, c2, c3), res| {
			let (r1, r2, r3) = res.lens();
			(
				c1.max(r1),
				c2.max(r2),
				c3.max(r3),
			)
		});

	// Print the successes!
	results.iter()
		.for_each(|x| x.print(c1, c2, c3));

	println!();
}

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
