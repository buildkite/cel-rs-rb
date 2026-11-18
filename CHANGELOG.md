# Changelog

## 0.4.0

Upgrade to the published [cel 0.16.0](https://crates.io/crates/cel/0.16.0)
from crates.io. This is a stable release, with no Git dependency or beta updater.
The [upstream comparison](https://github.com/cel-rust/cel-rust/compare/v0.15.0...v0.16.0)
is the full dependency change since 0.3.x.

### Breaking and behavior changes

- Ordering a NaN against any number with `<`, `<=`, `>`, or `>=` now returns
  `false`, in either operand position, instead of raising `CEL::ExecutionError`.
- `Program#references` excludes comprehension iteration/accumulator bindings and
  removes leading dots from absolute variable and function names.
- Map membership with an unusable key now returns `false`: `[] in {}` no longer
  raises.
- Optional indexing and field access suppress only missing-key and out-of-bounds
  errors.
- List addition now requires another list: `[1] + {'a': 2}` raises
  `CEL::ExecutionError` instead of appending the map's keys.
- Durations now accept ±315576000000.999999999 seconds (about 10,000 years). Ruby's
  floating-point seconds still cannot preserve every nanosecond at large magnitudes.

### Added

- `max_iterations:` on `CEL.compile` and `CEL::Program.compile`. The default `0`
  is unlimited; positive unsigned 64-bit integers bound total comprehension
  iterations across nested and sibling loops, with a fresh budget per execution.
- Per-call `timeout:` (finite, non-negative seconds) and `interrupt:` options on
  `Program#execute`, `#call`, and `#execute_with_context`, plus
  `CEL::CancellationToken.new`, `#cancel!`, and `#cancelled?` for cross-thread
  cancellation.
- Opt-in `extensions: [:sets]` for `sets.contains`, `sets.equivalent`, and
  `sets.intersects`. Lists are treated as sets: order and duplicates do not matter,
  and numeric elements compare by value.
- `CEL::Duration::MAX_NANOSECONDS` and `MIN_NANOSECONDS` expose CEL's exact
  inclusive duration limits as Ruby Integers.

### Fixed

- Failures on secondary Ruby threads now raise the documented `CEL::ParseError`,
  `CEL::ExecutionError`, and `CEL::TypeError`, rather than plain `RuntimeError`.
- Release replaced Ruby callbacks and their captured values instead of retaining
  every registration until the `CEL::Context` is collected. In-flight executions
  keep their original callbacks alive.

## 0.3.1

### Fixed

- Keep registered Ruby callback Procs and their captured values alive while their
  `CEL::Context` is reachable. Prevents use-after-free after the caller drops its
  own reference and Ruby runs garbage collection. Callbacks are also pinned during
  compaction, including when collection runs inside a callback.
