# Changelog



## [0.3.6](https://github.com/Blobfolio/brunch/releases/tag/v0.3.6) - 2022-09-22

### Changed

* Improved docs
* Miscellaneous code cleanup



## [0.3.5](https://github.com/Blobfolio/brunch/releases/tag/v0.3.5) - 2022-09-12

### New

* `BRUNCH_HISTORY=/path/to/file` env

### Changed

* Bump MSRV `1.63.0`
* Drop `serde` and `serde_json` dependencies
* `Bench::with_samples` now requires `u32` rather than `usize`
* Bench name lengths are now explicitly limited to `65,535`

### Deprecated

* `BRUNCH_DIR=/path/to/dir` env (use `BRUNCH_HISTORY` instead)



## [0.3.4](https://github.com/Blobfolio/brunch/releases/tag/v0.3.4) - 2022-09-09

### New

* `benches!(inline: …)`  macro variant for use inside a custom `main()`

### Changed

* Update dependencies



## [0.3.3](https://github.com/Blobfolio/brunch/releases/tag/v0.3.3) - 2022-09-06

### Changed

* Improved float handling;
* Minor code cleanup, refactoring;
* Drop `quantogram` dependency (it is now dev-only);



## [0.3.2](https://github.com/Blobfolio/brunch/releases/tag/v0.3.2) - 2022-08-23

### Changed

* One _final_ coloration tweak. (Sorry for the release spam!)



## [0.3.1](https://github.com/Blobfolio/brunch/releases/tag/v0.3.1) - 2022-08-23

### Changed

* Coloration tweaks;
* Improved float handling;
* Suppress _Change_ summary column when there aren't any;
* Warn if the same `Bench` name is submitted twice;

### Fixed

* env `NO_BRUNCH_HISTORY` should only apply when `=1`;
* Normalize whitespace in `Bench` names to prevent display weirdness;



## [0.3.0](https://github.com/Blobfolio/brunch/releases/tag/v0.3.0) - 2022-08-22

This release includes a number of improvements to the `Brunch` API, but also some **breaking changes**. Existing benchmarks will require a few (minor) adjustments when migrating from `0.2.x` to `0.3.x`.

First and foremost, `Bench::new` has been streamlined, and now takes the name as a single argument (rather than two). When migrating, just glue the two values back together, e.g. `"foo::bar", "baz(20)"` to `"foo::bar::baz(20)"`.

Each bench now runs until it has reached _either_ its sample or timeout limit, rather than running as many times as it can within a fixed time period. Existing benchmarks with `Bench::timed` will need to switch to `Bench::with_timeout` if that was used to extend the run, or removed if used to shorten it.

The execution methods have been cleaned up as well, and now come in three flavors:

| Method | Argument(s) | Description |
| ------ | ----------- | ----------- |
| `Bench::run` | `FnMut()->O` | For use with self-contained (argument-free) benchmarks. |
| `Bench::run_seeded` | `I: Clone`, `FnMut(I)->O` | For use with benchmarks that accept a single, cloneable argument. |
| `Bench::run_seeded_with` | `FnMut() -> I`, `FmMut(I)->O` | Also for benchmarks that accept one argument, but one that's easier to produce from a callback. |

`Bench::run` corresponds to `0.2.x`'s `Bench::with`, while `Bench::run_seeded*` is akin to `0.2.x`'s `Bench::with_setup*`.

There is no longer any explicit argument-as-reference version, but you can accomplish the same thing using `Bench::run_seeded`, like:

```rust
Bench::new("hello::world(&15)")
	.run_seeded(15, |s| hello::world(&s))
```

Time-tracking is now done per-method-call (rather than in batches), meaning the precision is now capped at the level of nanoseconds. This improves the results in a number of ways, but means _really fast_ methods won't chart in a meaningful way anymore.

If you need to compare _really fast_ things, it is recommended you perform some sort of iteration within the callback being benchmarked (to increase its runtime). Take a look at the main documentation for an example.

That's it! Apologies for the switch-up, but hopefully you'll agree the new layout is friendlier and more flexible than the old one. :)

### New

* Benches can now be constructed without the `benches!` macro if desired, using the new `Benches` struct. Refer to the main documentation for an example;
* `Bench::with_samples` (to set target sample limit);
* `Bench::with_timeout` (to set duration timeout);
* [`quantogram`](https://crates.io/crates/quantogram) and [`unicode-width`](https://crates.io/crates/unicode-width) have been added as dependencies;
* The environmental variable `BRUNCH_DIR` can be used to specify a location other than `std::env::temp_dir` for the history file.
* The environmental variable `NO_BRUNCH_HISTORY` can be used to disable run-to-run history altogether.

### Changed

* `Bench::new` now accepts name as a single argument instead of two;
* Improved statistical analysis, particularly in regards to outlier detection/removal;
* Improved visual display, particularly in regards to multibyte column layouts;
* Improved memory usage for seeded benchmarks;
* Time tracking is now capped at `nanoseconds`;

### Removed

* `Bench::timed` (see `Bench::with_timeout`);
* `Bench::with` (see `Bench::run`);
* `Bench::with_setup` (see `Bench::run_seeded` / `Bench::run_seeded_with`);
* `Bench::with_setup_ref` (see `Bench::run_seeded` / `Bench::run_seeded_with`);
* [`serde_derive`](https://crates.io/crates/serde_derive) dependency;



## [0.2.6](https://github.com/Blobfolio/brunch/releases/tag/v0.2.6) - 2022-06-18

### Misc

* Update dependencies;



## [0.2.5](https://github.com/Blobfolio/brunch/releases/tag/v0.2.5) - 2022-04-12

### Fixed

* Enable `num-traits` crate feature `i128` (needed for some targets).



## [0.2.4](https://github.com/Blobfolio/brunch/releases/tag/v0.2.4) - 2022-03-20

### Added

* `Bench::spacer` (for visual separation of results);



## [0.2.3](https://github.com/Blobfolio/brunch/releases/tag/v0.2.3) - 2022-03-15

### Misc

* Update dependencies;



## [0.2.2](https://github.com/Blobfolio/brunch/releases/tag/v0.2.2) - 2022-01-13

### Changed

* Updated docs;
* Cleaned up lint overrides;



## [0.2.1](https://github.com/Blobfolio/brunch/releases/tag/v0.2.1) - 2021-12-02

### Added

* A demo benchmark (`fn_fib`).

### Changed

* The list passed to the `brunch::benches!` macro may now include a trailing comma.



## [0.2.0](https://github.com/Blobfolio/brunch/releases/tag/v0.2.0) - 2021-10-21

### Added

* This changelog! Haha.

### Changed

* Use Rust edition 2021.
