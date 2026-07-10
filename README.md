# cel-rs-rb

[![Gem Version](https://img.shields.io/gem/v/cel-rs-rb.svg)](https://rubygems.org/gems/cel-rs-rb)

Ruby bindings for the Rust [`cel`](https://crates.io/crates/cel) crate, implemented with [Magnus](https://github.com/matsadler/magnus).

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

### API mapping

- `CEL.compile(source)` -> `CEL::Program`
- `CEL::Program.compile(source)` -> `CEL::Program`
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

Releases publish **precompiled native gems** for six platforms
(`x86_64-linux`, `aarch64-linux`, `x86_64-linux-musl`, `aarch64-linux-musl`,
`arm64-darwin`, `x86_64-darwin`) to
[rubygems.org](https://rubygems.org/gems/cel-rs-rb).

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

- Some CEL value variants (e.g. opaque custom Rust types) cannot be fully marshaled to Ruby and raise `CEL::TypeError`.
- This project targets broad CEL compatibility, but parity for every Rust-only extension point is still evolving.
