# cel-rs-rb

[![Gem Version](https://img.shields.io/gem/v/cel-rs-rb.svg)](https://rubygems.org/gems/cel-rs-rb)

Ruby bindings for the Rust [`cel`](https://crates.io/crates/cel) 0.16 crate, implemented with [Magnus](https://github.com/matsadler/magnus).

See the [changelog](CHANGELOG.md) for breaking changes and upgrade notes.

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
| [`strings`](https://docs.rs/cel/0.16.0/cel/extensions/fn.strings.html) | `'TacoCat'.lowerAscii()`, `'a,b'.split(',')` |
| [`lists`](https://docs.rs/cel/0.16.0/cel/extensions/fn.lists.html) | `lists.range(3)`, `[3, 1, 2].sort()`, `items.sortBy(x, x.score)` |
| [`math`](https://docs.rs/cel/0.16.0/cel/extensions/fn.math.html) | `math.greatest(1, 5, 3)`, `math.least([1, 5, 3])` |
| [`encoders`](https://docs.rs/cel/0.16.0/cel/extensions/fn.encoders.html) | `base64.encode(b'hi')`, `base64.decode('aGk=')` |
| [`sets`](https://docs.rs/cel/0.16.0/cel/extensions/fn.sets.html) | `sets.contains([1, 2], [2])`, `sets.equivalent([1, 1], [1])`, `sets.intersects([1], [2])` |

`CEL::Program.compile` accepts the same `extensions:` option. Names can be
symbols or strings; unknown or repeated names raise `ArgumentError`.

Extensions are disabled by default and registered per program, for both macro
expansion and execution. They work with bare `execute` or a default context,
including its variables and Ruby callbacks. They do not change other programs
or add functions to explicitly empty contexts. Ruby callbacks that conflict
with enabled extension functions raise `CEL::ExecutionError` during execution,
just like conflicts with standard CEL builtins.

### Parser recursion limit

The parser limits nesting to 96 levels. Operator chains count toward the limit
even without parentheses. This includes arithmetic chains, field access,
method calls, and indexing. Expressions that exceed the limit raise
`CEL::ParseError` during compilation. The Ruby API does not expose a setting
for this limit.

### Evaluation limits and cancellation

Set a comprehension-iteration budget when compiling, and a timeout in seconds
or cancellation token for each execution:

```ruby
program = CEL.compile("items.map(x, x * 2)", max_iterations: 1_000)
context = CEL::Context.build(items: [2, 5, 9])
program.execute(context, timeout: 0.05) # => [4, 10, 18]

token = CEL::CancellationToken.new
token.cancel! # Another Ruby thread can call this while evaluation runs.
program.execute(context, interrupt: token)
# raises CEL::ExecutionError: operation interrupted
```

`max_iterations` is an unsigned 64-bit integer; `0` (the default) means unlimited.
It counts total iterations across nested and sibling comprehensions, with a fresh
budget per execution, including concurrent executions and explicitly empty contexts.

`timeout: nil` and `interrupt: nil` disable interruption. Timeouts must be finite,
non-negative seconds within Rust's duration range. Each call gets a fresh deadline,
starting before the execution snapshot is built. Tokens are one-way: `cancelled?`
stays true after `cancel!`, and every execution sharing a token sees cancellation.
Use a new token for an independent run. When both options are supplied, either can
interrupt execution. `call` and `execute_with_context` accept the same options.

Checks are **cooperative**, performed on comprehension iterations (`all`, `exists`,
`map`, `filter`, etc.). They are not hard wall-clock timeouts or general cost limits.
Straight-line expressions, extension work outside comprehensions, and Ruby callbacks
are not preempted. Once observed, interruption or budget exhaustion fails the whole
evaluation with `CEL::ExecutionError`, even inside `||`, `&&`, or optional accessors.

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
- `CEL::Duration::MIN_NANOSECONDS`, `MAX_NANOSECONDS` -> exact inclusive duration bounds as Ruby Integers

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

Ruby's `CEL::Duration.new`, `total_seconds` / `to_f`, and display methods use
floating-point seconds, so they cannot represent every nanosecond at large magnitudes. Keep exact
duration arithmetic and `string(duration)` formatting inside CEL when precision matters.

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

Registering a function name again with `define_function` or `add_function`
replaces its callback for future executions. In-flight executions keep using
their original snapshot. Callbacks are kept alive and pinned against Ruby GC
compaction while registered or used by an in-flight execution.

## Development

### Tooling

Uses [mise](https://mise.jdx.dev/) for versions:

```toml
[tools]
ruby = "4.0"
rust = "1.96.0"
```

### Test + lint

```bash
bundle exec rake
```

This runs:

- `standard` (format/lint)
- native extension compile
- `rspec`

CI also runs the test suite on Ruby 3.3, Ruby 3.4, and Ruby 4.0 through Buildkite.

## Releasing

See [CHANGELOG.md](CHANGELOG.md) for release notes. Update it alongside each version bump.

Releases publish a generic `ruby` platform gem and **precompiled native gems**
for six platforms (`x86_64-linux`, `aarch64-linux`, `x86_64-linux-musl`,
`aarch64-linux-musl`, `arm64-darwin`, `x86_64-darwin`) to
[rubygems.org](https://rubygems.org/gems/cel-rs-rb). The generic gem keeps
RubyGems' latest-version metadata aligned; supported platforms should still
resolve to their matching precompiled native gem.

### Cutting a release

The gem version comes from `lib/cel/version.rb`, not from the tag, so bump the
file and tag the same version:

```bash
# 1. Bump CEL::VERSION in lib/cel/version.rb (e.g. "0.2.0")
# 2. Refresh Gemfile.lock's local gem version
bundle install

# 3. Commit the bump
git add lib/cel/version.rb Gemfile.lock
git commit -m "Release v0.2.0"

# 4. Tag that commit (the v-prefix triggers the release pipeline)
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

`Gemfile.lock` includes a local `cel-rs-rb` path entry, so commit its version
change alongside `lib/cel/version.rb`.

The `release:verify-tag` step fails the build if the tag and
`CEL::VERSION` disagree, so the tag (`v0.2.0`) must match `version.rb`
(`0.2.0`).
