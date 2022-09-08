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

```rust
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



## Installation

Add `brunch` to your `dev-dependencies` in `Cargo.toml`, like:

```yaml
[dev-dependencies]
brunch = "0.3.*"
```

Benchmarks should also be defined in `Cargo.toml`. Just be sure to set `harness = false` for each:

```yaml
[[bench]]
name = "encode"
harness = false
```

The following optional environmental variables are supported:

* `NO_BRUNCH_HISTORY=1`: don't save or load run-to-run history data;
* `BRUNCH_DIR=/some/directory`: save run-to-run history data to this folder instead of `std::env::temp_dir`;



## Usage

The heart of `Brunch` is the `Bench` struct, which defines a single benchmark. There isn't much configuration required, but each `Bench` has the following:

| Data | Description | Default |
| ---- | ----------- | ------- |
| Name | A unique identifier. This is arbitrary, but works best as a string representation of the method itself, like `foo::bar(10)` | |
| Samples | The number of samples to collect. | 2500 |
| Timeout | A cutoff time to keep it from running forever. | 10 seconds |
| Method | A method to run over and over again! | |

The struct uses builder-style methods to allow everything to be set in a single chain. You always need to start with `Bench::new` and end with one of the runner methods — `Bench::run`, `Bench::run_seeded`, or `Bench::run_seeded_with`. If you want to change the sample or timeout limits, you can add `Bench::with_samples` or `Bench::with_timeout` in between.

There is also a special `Bench::spacer` method that can be used to inject a linebreak into the results. See below for an example.

### Examples

In terms of running benchmarks, the simplest approach is to use the provided `benches` macro. That generates the required `main()` method, runs all the benches, and prints the results automatically.

```rust
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

If you prefer to handle things manually — for example to perform one-time setup or resolve lifetime conflicts — you'll need to use the `Benches` struct directly instead.

At any rate, it's easy:

```rust
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

```text
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



## License

See also: [CREDITS.md](CREDITS.md)

Copyright © 2022 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004
    
    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>
    
    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.
    
    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
    
    0. You just DO WHAT THE FUCK YOU WANT TO.
