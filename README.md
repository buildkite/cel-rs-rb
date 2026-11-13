# cel-rs-rb

[![Gem Version](https://img.shields.io/gem/v/cel-rs-rb.svg)](https://rubygems.org/gems/cel-rs-rb)

Ruby bindings for the Rust [`cel`](https://crates.io/crates/cel) crate,
implemented with [Magnus](https://github.com/matsadler/magnus).

## Goals

- Ruby-first API over the Rust CEL engine
- Close semantic compatibility with Rust `cel::Program` + `cel::Context`
- Ruby variable and function interop
- Thread-safe concurrent execution
- GVL released while CEL evaluation runs

## Installation

```ruby
gem "cel-rs-rb"
```

## Usage

```ruby
require "cel"

context = CEL::Context.build(user: {"name" => "Ada"}, scores: [10, 20, 30]) do |ctx|
  ctx.define_function("sum") { |*args| args.sum }
end

program = CEL.compile("sum(scores[0], scores[1], scores[2])")
program.execute(context)
# => 60
```

### Optional values

Use optional access to handle missing fields, and `orValue` to supply a default:

```ruby
context = CEL::Context.build(user: {})
CEL.compile("user.?name.orValue('unknown')").execute(context)
# => "unknown"

context = CEL::Context.build(user: {"name" => "Ada"})
CEL.compile("user.?name.optMap(n, n.size()).orValue(0)").execute(context)
# => 3
```

Unwrap optional values inside CEL with `orValue` or `value()` before returning
them to Ruby. Returning an optional directly raises `CEL::TypeError`; it is not
automatically converted to its contained value or `nil`.

### `max` and `min`

Default contexts include `max` and `min`. They take a list or positional arguments and
return `nil` when given no values:

```ruby
CEL.compile("max(2, 9, -3)").execute   # => 9
CEL.compile("min([2, 9, -3])").execute # => -3
CEL.compile("max([])").execute         # => nil
```

### Extension libraries

Opt into upstream CEL extension libraries when compiling a program:

```ruby
program = CEL.compile(
  "[label.lowerAscii(), scores.sort(), math.greatest(scores), base64.encode(b'hi')]",
  extensions: [:strings, :lists, :math, :encoders]
)
program.execute(CEL::Context.build(label: "TacoCat", scores: [2, 9, -3]))
# => ["tacocat", [-3, 2, 9], 9, "aGk="]
```

| Library | Examples |
| --- | --- |
| [`strings`](https://docs.rs/cel/0.15.0/cel/extensions/fn.strings.html) | `'TacoCat'.lowerAscii()`, `'a,b'.split(',')` |
| [`lists`](https://docs.rs/cel/0.15.0/cel/extensions/fn.lists.html) | `lists.range(3)`, `[3, 1, 2].sort()`, `items.sortBy(x, x.score)` |
| [`math`](https://docs.rs/cel/0.15.0/cel/extensions/fn.math.html) | `math.greatest(1, 5, 3)`, `math.least([1, 5, 3])` |
| [`encoders`](https://docs.rs/cel/0.15.0/cel/extensions/fn.encoders.html) | `base64.encode(b'hi')`, `base64.decode('aGk=')` |

`CEL::Program.compile` accepts the same `extensions:` option. Names can be
symbols or strings; unknown or repeated names raise `ArgumentError`.

Extensions are disabled by default and registered per program, for both macro
expansion and execution. They work with bare `execute` or a default context,
including its variables and Ruby callbacks. They do not change other programs
or add functions to explicitly empty contexts. Ruby callbacks that conflict
with enabled extension functions raise `CEL::ExecutionError` during execution,
just like conflicts with standard CEL builtins.

### Evaluation limits and cancellation

Set a comprehension-iteration budget when compiling, and an optional timeout
(in seconds) or cancellation token for each execution:

```ruby
program = CEL.compile("items.map(x, x * 2)", max_iterations: 1_000)
context = CEL::Context.build(items: [2, 5, 9])
program.execute(context, timeout: 0.05) # => [4, 10, 18]

token = CEL::CancellationToken.new
token.cancel! # Another Ruby thread can call this while evaluation runs.
program.execute(context, interrupt: token)
# raises CEL::ExecutionError: operation interrupted
```

`max_iterations` is a non-negative 64-bit integer; `0` (the default) disables
the budget. The budget counts total iterations across nested and sibling
comprehensions, with a fresh budget for each execution, including concurrent
executions of one program. It also applies to explicitly empty contexts.

`timeout: nil` and `interrupt: nil` disable interruption by default. Timeouts
must be finite, non-negative seconds within Rust's duration range. A timeout
starts before building the execution snapshot; each call gets a fresh deadline.
Tokens are one-way: `cancelled?` stays true after `cancel!`, and every evaluation
sharing that token sees the cancellation. Use a new token for an independent run.
When both options are supplied, either can interrupt execution. `call` and
`execute_with_context` accept the same execution options.

These checks are **cooperative**, performed on comprehension iterations
(`all`, `exists`, `map`, `filter`, etc.). They are not hard wall-clock timeouts
or a general CEL cost limit. Straight-line expressions, extension work outside
comprehensions, and Ruby callbacks are not preempted; a callback must return
before CEL can observe cancellation. Once observed, interruption or budget
exhaustion fails the entire evaluation with `CEL::ExecutionError`, even inside
`||`, `&&`, or optional accessors.

### API mapping

- `CEL.compile(source, extensions: [], max_iterations: 0)` -> `CEL::Program`
- `CEL::Program.compile(source, extensions: [], max_iterations: 0)` -> `CEL::Program`
- `CEL::Program#execute(context = nil, timeout: nil, interrupt: nil)` -> Ruby value
- `CEL::Program#references` -> `{ "variables" => [...], "functions" => [...] }`
- `CEL::CancellationToken.new`, `#cancel!`, `#cancelled?` -> cooperative cancellation
- `CEL::Context.new(empty = false)` -> context with builtins (`false`) or empty (`true`)
- `CEL::Context#add_variable(name, value)`
- `CEL::Context#define_function(name) { |*args| ... }`
- `CEL::Duration.new(seconds)` -> duration value for context variables and duration results

## Type support

### Ruby -> CEL

- `nil` -> `null`
- `true/false` -> `bool`
- `Integer` -> `int`
- `Float` -> `double`
- `String` / `Symbol` -> `string`
- binary `String` (`ASCII-8BIT`, e.g. `"abc".b`) -> `bytes`
- `Time` -> `timestamp`
- `CEL::Duration` -> `duration`
- `Array` -> `list`
- `Hash` with keys `String|Symbol|Integer|Boolean` -> `map`

### CEL -> Ruby

- `null` -> `nil`
- scalar primitives map naturally
- `bytes` -> binary Ruby `String`
- `timestamp` -> `Time`
- `duration` -> `CEL::Duration`
- `list` -> `Array`
- `map` -> `Hash`

## Error classes

- `CEL::Error`
- `CEL::ParseError`
- `CEL::ExecutionError`
- `CEL::TypeError`

## Thread safety and concurrency

- Compiled `CEL::Program` instances can be shared and executed concurrently from
  multiple Ruby threads with separate execution contexts.
- Each thread may pass its own `CEL::Context`; context data is mutex-protected
  and copied into an immutable execution snapshot for each run.
- CEL evaluation runs with the Ruby GVL released, so other Ruby threads can keep
  progressing while a program executes.
- Ruby callbacks from CEL functions temporarily re-acquire the GVL before
  invoking Ruby code.

## Development

### Tooling

Uses [mise](https://mise.jdx.dev/) for versions:

```toml
[tools]
ruby = "4.0"
rust = "1.96.0"
```

Install the configured toolchains and Ruby dependencies:

```bash
mise install
mise run deps
```

Native compilation also requires a C toolchain and libclang. CI installs the
Linux system dependencies with `mise run ci:install-system-deps`.

### Test + lint

```bash
mise run test
mise run clippy
```

`mise run test` runs:

- `standard` (format/lint)
- native extension compile
- `rspec`

`mise run clippy` checks the Rust code with Clippy.

CI also runs the test suite on Ruby 3.3, Ruby 3.4, and Ruby 4.0 through Buildkite.

### Nightly snapshots

Nightly betas pin upstream's default branch and use `X.Y.Z.beta.YYYYMMDD.N`
(next gem release, UTC date, daily counter).

Prepare a snapshot, then test and open an update PR:

```bash
mise run beta:update
mise run test
mise run clippy
```

Set `BETA_BASE_VERSION=0.5.0` when starting a new gem release line.

Buildkite produces tested native and source `.gem` artifacts for prerelease PRs;
download them from that build to review. Nothing is scheduled or published
automatically. Merging, tagging, and publishing require human approval.

Source gems require Rust and Git; transitive dependencies resolve at install time.

## Releasing

Releases publish a generic `ruby` platform gem and **precompiled native gems**
for six platforms (`x86_64-linux`, `aarch64-linux`, `x86_64-linux-musl`,
`aarch64-linux-musl`, `arm64-darwin`, `x86_64-darwin`) to
[rubygems.org](https://rubygems.org/gems/cel-rs-rb). The generic gem keeps
RubyGems' latest-version metadata aligned; supported platforms should still
resolve to their matching precompiled native gem.

### Human-authorized publication

Merging an update PR, creating/pushing a version tag, and publishing each release
require explicit human authorization. A successful beta build grants none of
these permissions.

After review and authorization to tag the selected commit, the tag must be
`v` followed by its exact `CEL::VERSION`, for example
`v0.4.0.beta.20261111.1`. `release:verify-tag` rejects a mismatch.
The release pipeline builds and tests its packages, then pauses at **Approve
publication**. A human with Buildkite unblock permission must approve that
specific build before the RubyGems OIDC publish step can run.

A manual release-pipeline build without a tag, or a build with `DRY_RUN=true`,
only builds/tests artifacts: it skips both the approval and publication steps.
Before stable promotion, choose the gem's stable version and released cel crate
dependency deliberately, update both lockfiles, and repeat review and validation.

## Compatibility notes

- The beta includes the full upstream range from cel 0.15.0 through the pinned
  snapshot, not just #339. `references` now excludes comprehension bindings
  (such as `x` in `items.map(x, x + offset)`) and removes leading dots from
  absolute names. Consumers should supply only the remaining external variables.
- Map membership with an unusable key (such as `[] in {'a': 1}`) now returns
  `false`. Optional indexing suppresses missing-key/out-of-bounds errors only;
  invalid operations such as `[1][?'bad'].hasValue()` raise `CEL::ExecutionError`.
- Existing Ruby APIs remain available, and duplicate extension names still
  raise `ArgumentError` even though upstream library registration is now idempotent.
  New upstream bindings/network libraries and configurable list-range sizes
  are not exposed by this beta.
- Compilation uses the standard CEL environment, including optional access syntax (`user.?name`) and the `optMap` / `optFlatMap` macros.
- The upstream strings, lists, math, and encoders extension libraries are opt-in through the `extensions:` compile option.
- cel 0.15 no longer includes `max` and `min` by default, so default contexts register them for compatibility with cel 0.13. See [`max` and `min`](#max-and-min).
- When upgrading from cel 0.13, replace list/map `.contains(value)` expressions with `value in collection`; `.contains()` now only accepts strings. Map literals with duplicate keys raise `CEL::ExecutionError`, and string `size()` counts Unicode code points rather than bytes. See the [cel 0.15 release notes](https://github.com/cel-rust/cel-rust/releases/tag/v0.15.0) for further CEL semantic changes.
- `string(bytes)` now rejects invalid UTF-8. Numeric conversions reject out-of-range results, and parsed timestamps and durations must fit CEL's allowed ranges.
- `all()` and `exists()` can now absorb an earlier evaluation error when a later element determines the result. For example, `[2, 1].exists(e, 6 / (2 - e) == 6)` returns `true` despite the first element's division by zero.
- Ruby callbacks cannot shadow standard CEL builtins (such as `size`) in a default context; the compatibility functions `max` and `min` are exceptions. Conflicts raise `CEL::ExecutionError` when `Program#execute(context)` builds the execution snapshot, even if the expression does not call the conflicting function. Use `CEL::Context.new(true)` or `CEL::Context.build(empty: true)` to define functions without builtins. Registering the same custom callback name again replaces the previous callback.
- Some CEL value variants (e.g. opaque custom Rust types) cannot be fully marshaled to Ruby and raise `CEL::TypeError`.
- This project targets broad CEL compatibility, but parity for every Rust-only extension point is still evolving.
