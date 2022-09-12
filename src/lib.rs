/*!
# Brunch

[![Documentation](https://docs.rs/brunch/badge.svg)](https://docs.rs/brunch/)
[![Changelog](https://img.shields.io/crates/v/brunch.svg?label=Changelog&color=9cf)](https://github.com/Blobfolio/brunch/blob/master/CHANGELOG.md)
[![crates.io](https://img.shields.io/crates/v/brunch.svg)](https://crates.io/crates/brunch)
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
* `BRUNCH_HISTORY=/some/file.whatever`: save run-to-run history data to this specific file path (instead of [`std::env::temp_dir`]/\_\_brunch.last);



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

The [`benches`] macro is the easiest way to run `Brunch` benchmarks.

Simply pass a comma-separated list of all the [`Bench`](crate::Bench) objects you want to run, and it will handle the setup, running, tabulation, and give you a nice summary at the end.

By default, this macro will generate the `main()` entrypoint too, but you can suppress this by adding "inline:" as the first argument.

Anyhoo, the default usage would look something like the following:

```no_run
use brunch::{Bench, benches};

// Example benchmark adding 2+2.
fn callback() -> Option<usize> { 2_usize.checked_add(2) }

// Example benchmark multiplying 2x2.
fn callback2() -> Option<usize> { 2_usize.checked_mul(2) }

// Let the macro handle everything for you.
benches!(
    Bench::new("usize::checked_add(2)")
        .run(callback),

    Bench::new("usize::checked_mul(2)")
        .run(callback2),
);
```

When declaring your own main entrypoint, you need to add "inline:" as the first argument. The list of [`Bench`](crate::Bench) instances follow as usual after that.

```no_run
use brunch::{Bench, benches};

/// # Custom Main.
fn main() {
    // A typical use case for the "inline" variant would be to declare
    // an owned variable for a benchmark that needs to return a reference
    // (to e.g. keep Rust from complaining about lifetimes).
    let v = vec![0_u8, 1, 2, 3, 4, 5];

    // The macro call goes here!
    benches!(
        inline:

        Bench::new("vec::as_slice()").run(|| v.as_slice()),
    );

    // You can also do other stuff afterwards if you want.
    eprintln!("Done!");
}
```

For even more control over the flow, skip the macro and just use [`Benches`](crate::Benches) directly.



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
mod math;
mod stats;
pub(crate) mod util;



pub use bench::{
	Bench,
	Benches,
};
pub use error::BrunchError;
pub(crate) use math::Abacus;
pub(crate) use stats::{
	History,
	Stats,
};



/// # Minimum Number of Samples.
pub(crate) const MIN_SAMPLES: u32 = 100;
