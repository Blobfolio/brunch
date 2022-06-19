# Brunch

[![Documentation](https://docs.rs/brunch/badge.svg)](https://docs.rs/brunch/)
[![crates.io](https://img.shields.io/crates/v/brunch.svg)](https://crates.io/crates/brunch)
[![Build Status](https://github.com/Blobfolio/brunch/workflows/Build/badge.svg)](https://github.com/Blobfolio/brunch/actions)
[![Dependency Status](https://deps.rs/repo/github/blobfolio/brunch/status.svg)](https://deps.rs/repo/github/blobfolio/brunch)

`Brunch` is a very simple Rust micro-benchmark runner inspired by [`easybench`](https://crates.io/crates/easybench). It has roughly a million times fewer dependencies than [`criterion`](https://crates.io/crates/criterion), does not require nightly, and maintains a "last run" state so can show relative changes benchmark-to-benchmark. The formatting is also quite pretty.

As with all Rust benchmarking, there are a lot of caveats, and results might be artificially fast or slow. For best resuilts, build optimized, avoid heavy setup contexts, and test different bench setups to find the most "honest" representation.

In theory, this library can reach pico-second scales (it clocks increasingly large batches and divides accordingly), but background noise and setup overhead will likely prevent times getting quite as low as they might "actually" be. It can go as long as milliseconds, but might require increased time limits to reach sufficient samples in such cases.



## Installation

Add `brunch` to your `dev-dependencies` in `Cargo.toml`, like:

```
[dev-dependencies]
brunch = "0.2.*"
```

Benchemarks are also defined in `Cargo.toml` the usual way. Just be sure to set `harness = false`:

```
[[bench]]
name = "encode"
harness = false
```



## Usage

Setup is currently simple if primitive, requiring you drop a call to `brunch::benches!()` in the benchmark file. It will generate a `main()` method, run the supplied benchmarks, and give you the results.

An example bench file would look something like:

```rust
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

The `Bench` struct represents a benchmark. It takes two label arguments intended to represent a shared base (for the included benchmarks) and the unique bit, usually a method/value.

By default, each benchmark will run for approximately three seconds. This can be changed using the chained `Bench::timed` method as shown above.

There are currently three styles of callback:

| Method | Signature | Description |
| ------ | --------- | ----------- |
| `with` | `FnMut() -> O` | Execute a self-contained callback. |
| `with_setup` | `FnMut(I) -> O` | Execute a callback seeded with a (cloneable) value. |
| `with_setup_ref` | `FnMut(&I) -> O` | Execute a callback seeded with a referenced value. |

The benchmarks are run in the order entered, and their results likewise follow that same ordering.

If you want to break up the results visually, you can add a call to `Bench::spacer` anywhere you want a break to occur, like:

```rust
brunch::benches!(
    Bench::new("dactyl::NiceU8", "from(0)")
        .with(|| NiceU8::from(0_u8)),

    Bench::new("dactyl::NiceU8", "from(18)")
        .with(|| NiceU8::from(18_u8)),

    Bench::spacer(),

    Bench::new("dactyl::NiceU16", "from(0)")
        .with(|| NiceU16::from(0_u16)),

    Bench::new("dactyl::NiceU16", "from(18)")
        .with(|| NiceU16::from(18_u16))
);
```

The above would give you a result readout like:

```text
dactyl::NiceU8::from(0)     2.36 ns   
dactyl::NiceU8::from(18)    1.92 ns   

dactyl::NiceU16::from(0)    4.46 ns   
dactyl::NiceU16::from(18)   4.47 ns
```



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
