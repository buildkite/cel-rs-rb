# Changelog

## 0.4.0

Upgrade to the published [cel 0.16.0](https://crates.io/crates/cel/0.16.0)
from crates.io. This is a stable release, with no Git dependency or beta updater.
The [upstream comparison](https://github.com/cel-rust/cel-rust/compare/v0.15.0...v0.16.0)
is the full dependency change since 0.3.x.

### Breaking and behavior changes

- Ordering a NaN against any number with `<`, `<=`, `>`, or `>=` now returns
  `false`, in either operand position, instead of raising `CEL::ExecutionError`.
  Comparisons against non-numbers still fail. Check for NaN explicitly if it
  should invalidate a rule rather than produce `false`.
- `Program#references` excludes comprehension iteration/accumulator bindings and
  removes leading dots from absolute variable and function names. For example,
  `items.map(x, x + .offset)` reports only `items` and `offset` as variables.
  `optMap` / `optFlatMap` expansions no longer report a synthetic `optional`
  variable, and report qualified function names such as `optional.of` instead
  of the former member-call names. Update consumers of the reference lists;
  namespaced function/type names may still appear as variables.
- Map membership with an unusable key now returns `false`: `[] in {}` no longer
  raises. Whole-number numeric keys still match equivalent integer keys.
- Optional indexing and field access suppress only missing-key and out-of-bounds
  errors. Invalid operations such as `{}[?[]]`, `[1][?'a']`, and `1.?a` now raise
  `CEL::ExecutionError` instead of producing an empty optional. Validate types
  rather than using optional access to hide invalid operations.
- List addition now requires another list: `[1] + {'a': 2}` raises
  `CEL::ExecutionError` instead of appending the map's keys. Use an explicit
  map comprehension to obtain a key list before concatenation.
- Durations now accept ±315576000000.999999999 seconds (about 10,000 years),
  instead of an `i64` nanosecond range (about 292 years). Parsing, arithmetic,
  and timestamp subtraction can now succeed where they previously failed.
  Decimal components are parsed exactly to nanoseconds, truncating sub-nanosecond
  fractions toward zero; `string(duration('4.02us'))` is now `"0.00000402s"`.
  Ruby duration conversion supports the widened range without clamping. Ruby's
  floating-point seconds still cannot preserve every nanosecond at large magnitudes.
- Upstream dispatch now uses coarse argument types, accepts type parameters, and
  rejects overlapping overload declarations. Optional methods reject non-optional
  receivers during dispatch, so invalid-call error messages can change. Ruby has
  no typed-overload declaration API: callback replacement by name and rejection
  of callbacks conflicting with builtins remain unchanged.

### Added

- `max_iterations:` on `CEL.compile` and `CEL::Program.compile`. The default `0`
  is unlimited; positive unsigned 64-bit integers bound total comprehension
  iterations across nested and sibling loops, with a fresh budget per execution.
- Per-call `timeout:` (finite, non-negative seconds) and `interrupt:` options on
  `Program#execute`, `#call`, and `#execute_with_context`, plus
  `CEL::CancellationToken.new`, `#cancel!`, and `#cancelled?` for cross-thread
  cancellation. Defaults remain uninterrupted. Cancellation is one-way.
- Budget exhaustion and interruption raise `CEL::ExecutionError` and cannot be
  swallowed by boolean or optional expressions. Checks are cooperative at
  comprehension iterations; they do not preempt Ruby callbacks, straight-line
  expressions, or extension work outside comprehensions.
- Opt-in `extensions: [:sets]` for `sets.contains`, `sets.equivalent`, and
  `sets.intersects`. Lists are treated as sets: order and duplicates do not matter,
  and numeric elements compare by value. Extensions remain per-program and disabled
  by default. Repeated extension names still raise `ArgumentError` in Ruby even
  though upstream library registration is now idempotent.

### Upstream changes outside the Ruby API

- Bindings (`cel.bind` / `cel.@block`), network, and two-variable comprehensions
  are new opt-in Rust extensions, not enabled or exposed by this gem. The change
  to bind iteration variables before loop conditions and the safer append/map
  accumulator optimizations require custom macros/ASTs or those extensions to
  change behavior; Ruby exposes neither custom macros nor AST construction.
- Configurable `lists.range` limits remain Rust-only; the Ruby lists extension
  keeps its 1,000,000-element default. Overload result declarations, `Context`'s
  execution frame, and restored unwind-safety traits do not add Ruby APIs.
- Lexer/parser fixes target the optional Pratt parser; this gem continues to
  compile with upstream's default ANTLR parser. Benchmark, conformance, example,
  CI, and documentation changes do not change the gem's public API.

### Fixed

- Failures on secondary Ruby threads now raise the documented `CEL::ParseError`,
  `CEL::ExecutionError`, and `CEL::TypeError`, rather than plain `RuntimeError`.
- Release replaced Ruby callbacks and their captured values instead of retaining
  every registration until the `CEL::Context` is collected. In-flight executions
  keep their original callbacks alive and pinned against GC compaction until
  execution finishes, including when execution raises an error or its Ruby thread
  is interrupted or killed.
- Include `Cargo.lock` in source gems so native builds use the checked-in Rust dependency versions.

## 0.3.1

### Fixed

- Keep registered Ruby callback Procs and their captured values alive while their
  `CEL::Context` is reachable. Prevents use-after-free after the caller drops its
  own reference and Ruby runs garbage collection. Callbacks are also pinned during
  compaction, including when collection runs inside a callback.
