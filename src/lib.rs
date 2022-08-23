/*!
# Brunch

[![Documentation](https://docs.rs/brunch/badge.svg)](https://docs.rs/brunch/)
[![crates.io](https://img.shields.io/crates/v/brunch.svg)](https://crates.io/crates/brunch)
[![Build Status](https://github.com/Blobfolio/brunch/workflows/Build/badge.svg)](https://github.com/Blobfolio/brunch/actions)
[![Dependency Status](https://deps.rs/repo/github/blobfolio/brunch/status.svg)](https://deps.rs/repo/github/blobfolio/brunch)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](https://github.com/Blobfolio/brunch)



`Brunch` is a very simple Rust micro-benchmark runner inspired by [`easybench`](https://crates.io/crates/easybench). It has roughly a million times fewer dependencies than [`criterion`](https://crates.io/crates/criterion), does not require nightly, and maintains a "last run" state so can show relative changes benchmark-to-benchmark.

(The formatting is also quite pretty.)

As with all Rust benchmarking, there are a lot of caveats, and results might be artificially fast or slow. For best results:
* Build optimized;
* Collect lots of samples;
* Repeat identical runs to get a feel for the natural variation;

`Brunch` cannot measure time below the level of a nanosecond, so if you're trying to benchmark methods that are _really_ fast, you may need to wrap them in a method that runs through several iterations at once. For example:

```no_run
use brunch::Bench;

///# Generate Strings to Test.
fn string_seeds() -> Vec<String> {
    (0..10_000_usize).into_iter()
        .map(|i| "x".repeat(i))
        .collect()
}

///# Generate Strings to Test.
fn byte_seeds() -> Vec<Vec<u8>> {
    (0..10_000_usize).into_iter()
        .map(|i| "x".repeat(i).into_bytes())
        .collect()
}

brunch::benches!(
    Bench::new("String::len(_)")
        .run_seeded_with(string_seeds, |vals| {
            let mut len: usize = 0;
            for v in vals {
                len += v.len();
            }
            len
        }),
    Bench::new("Vec::len(_)")
        .run_seeded_with(byte_seeds, |vals| {
            let mut len: usize = 0;
            for v in vals {
                len += v.len();
            }
            len
        }),
);
```



## Cargo.toml

Benchmarks are defined the usual way. Just be sure to set `harness = false`:

```ignore
[[bench]]
name = "encode"
harness = false
```

The following optional environmental variables are supported:

* `NO_BRUNCH_HISTORY=1`: don't save or load run-to-run history data;
* `BRUNCH_DIR=/some/directory`: save run-to-run history data to this folder instead of [`std::env::temp_dir`];



## Usage

The heart of `Brunch` is the [`Bench`] struct, which defines a single benchmark. There isn't much configuration required, but each [`Bench`] has the following:

| Data | Description | Default |
| ---- | ----------- | ------- |
| Name | A unique identifier, ideally a string representation of the call itself, like `foo::bar(10)` | |
| Samples | The number of samples to collect. | 2500 |
| Timeout | A cutoff time to keep it from running forever. | 10 seconds |
| Method | A method to run over and over again! | |

The struct uses builder-style methods to allow everything to be set in a single chain. You always need to start with [`Bench::new`] and end with one of the runner methods — [`Bench::run`], [`Bench::run_seeded`], or [`Bench::run_seeded_with`]. If you want to change the sample or timeout limits, you can add [`Bench::with_samples`] or [`Bench::with_timeout`] in between.

There is also a special [`Bench::spacer`] method that can be used to inject a linebreak into the results. See below for an example.

### Examples

In terms of running benchmarks, the simplest approach is to use the provided [`benches`] macro. That generates the required `main()` method, runs all the benches, and prints the results automatically.

```no_run
use brunch::Bench;
use dactyl::NiceU8;

/// # Silly seed method.
fn max_u8() -> u8 { u8::MAX }

brunch::benches!(
    // Self-contained bench.
    Bench::new("dactyl::NiceU8::from(0)")
        .run(|| NiceU8::from(0_u8)),

    // Clone-seeded bench.
    Bench::new("dactyl::NiceU8::from(18)")
        .run_seeded(18_u8, |num| NiceU8::from(num)),

    // An example of a spacer, which just adds a line break.
    Bench::spacer(),

    // Callback-seeded bench.
    Bench::new("dactyl::NiceU8::from(101)")
        .run_seeded_with(max_u8, |num| NiceU8::from(num)),
);
```

If you prefer to handle things manually, you'll need to use the [`Benches`] struct instead. It's pretty easy too:

```no_run
use brunch::{Benches, Bench};
use dactyl::NiceU8;

fn main() {
    // Do any setup you want.
    println!("This prints before any time-consuming work happens!");

    // Initialize a mutable `Benches`.
    let mut benches = Benches::default();

    // Push each `Bench` you have to it, one at a time (or use
    // `benches.extend([Bench1, Bench2, ...])` to do many at once).
    benches.push(
        Bench::new("dactyl::NiceU8::from(0)").run(|| NiceU8::from(0_u8))
    );

    // Call the `finish` method to crunch and print the results.
    benches.finish();

    // Do something else if you want to!
}
```



## Interpreting Results

If you run the example benchmark for this crate, you should see a summary like the following:

```ignore
Method                         Mean    Change        Samples
------------------------------------------------------------
fibonacci_recursive(30)     2.22 ms    +1.02%    2,408/2,500
fibonacci_loop(30)         56.17 ns       ---    2,499/2,500
```

The _Method_ column speaks for itself, but the numbers deserve a little explanation:

| Column | Description |
| ------ | ----------- |
| Mean | The adjusted, average execution time for a _single_ run, scaled to the most appropriate time unit to keep the output tidy. |
| Change | The relative difference between this run and the last run, if more than two standard deviations. |
| Samples | The number of valid/total samples, the difference being outliers (5th and 95th quantiles) excluded from consideration. |
*/

#![deny(unsafe_code)]

#![warn(
	clippy::filetype_is_file,
	clippy::integer_division,
	clippy::needless_borrow,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::suboptimal_flops,
	clippy::unneeded_field_pattern,
	macro_use_extern_crate,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![allow(
	clippy::module_name_repetitions,
	clippy::needless_doctest_main,
	clippy::redundant_pub_crate,
)]

mod bench;
mod error;
#[macro_use] mod macros;
mod stats;
pub(crate) mod util;



pub use bench::{
	Bench,
	Benches,
};
pub use error::BrunchError;
pub(crate) use stats::{
	History,
	Stats,
};



/// # Minimum Number of Samples.
pub(crate) const MIN_SAMPLES: usize = 100;
