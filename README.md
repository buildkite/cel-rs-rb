# cel-rs-rb

[![Gem Version](https://img.shields.io/gem/v/cel-rs-rb.svg)](https://rubygems.org/gems/cel-rs-rb)

Ruby bindings for the Rust [`cel`](https://crates.io/crates/cel) 0.15 crate, implemented with [Magnus](https://github.com/matsadler/magnus).

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
| `strings` | `'TacoCat'.lowerAscii()`, `'a,b'.split(',')` |
| `lists` | `lists.range(3)`, `[3, 1, 2].sort()`, `items.sortBy(x, x.score)` |
| `math` | `math.greatest(1, 5, 3)`, `math.least([1, 5, 3])` |
| `encoders` | `base64.encode(b'hi')`, `base64.decode('aGk=')` |

`CEL::Program.compile` accepts the same `extensions:` option. Names can be
symbols or strings; unknown or repeated names raise `ArgumentError`.

Extensions are disabled by default and registered per program, for both macro
expansion and execution. They work with bare `execute` or a default context,
including its variables and Ruby callbacks. They do not change other programs
or add functions to explicitly empty contexts. Ruby callbacks that conflict
with enabled extension functions raise `CEL::ExecutionError` during execution,
just like conflicts with standard CEL builtins.

### API mapping

- `CEL.compile(source, extensions: [])` -> `CEL::Program`
- `CEL::Program.compile(source, extensions: [])` -> `CEL::Program`
- `CEL::Program#execute(context = nil)` -> Ruby value
- `CEL::Program#references` -> `{ "variables" => [...], "functions" => [...] }`
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

## Compatibility notes

- Compilation uses the standard CEL environment, including optional access syntax (`user.?name`) and the `optMap` / `optFlatMap` macros.
- The upstream strings, lists, math, and encoders extension libraries are opt-in through the `extensions:` compile option.
- cel 0.15 no longer includes `max` and `min` by default, so default contexts register them for compatibility with cel 0.13. See [`max` and `min`](#max-and-min).
- When upgrading from cel 0.13, replace list/map `.contains(value)` expressions with `value in collection`; `.contains()` now only accepts strings. Map literals with duplicate keys raise `CEL::ExecutionError`, and string `size()` counts Unicode code points rather than bytes. See the [cel 0.15 release notes](https://github.com/cel-rust/cel-rust/releases/tag/v0.15.0) for further CEL semantic changes.
- `string(bytes)` now rejects invalid UTF-8. Numeric conversions reject out-of-range results, and parsed timestamps and durations must fit CEL's allowed ranges.
- `all()` and `exists()` can now absorb an earlier evaluation error when a later element determines the result. For example, `[2, 1].exists(e, 6 / (2 - e) == 6)` returns `true` despite the first element's division by zero.
- Ruby callbacks cannot shadow standard CEL builtins (such as `size`) in a default context; the compatibility functions `max` and `min` are exceptions. Conflicts raise `CEL::ExecutionError` when `Program#execute(context)` builds the execution snapshot, even if the expression does not call the conflicting function. Use `CEL::Context.new(true)` or `CEL::Context.build(empty: true)` to define functions without builtins. Registering the same custom callback name again replaces the previous callback.
- Some CEL value variants (e.g. opaque custom Rust types) cannot be fully marshaled to Ruby and raise `CEL::TypeError`.
- This project targets broad CEL compatibility, but parity for every Rust-only extension point is still evolving.
