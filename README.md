# cel-rs-rb

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

## Type support

### Ruby -> CEL

- `nil` -> `null`
- `true/false` -> `bool`
- `Integer` -> `int`
- `Float` -> `double`
- `String` / `Symbol` -> `string`
- `Array` -> `list`
- `Hash` with keys `String|Symbol|Integer|Boolean` -> `map`

### CEL -> Ruby

- `null` -> `nil`
- scalar primitives map naturally
- `list` -> `Array`
- `map` -> `Hash`

## Error classes

- `CEL::Error`
- `CEL::ParseError`
- `CEL::ExecutionError`
- `CEL::TypeError`

## Thread safety and concurrency

- Context data is mutex-protected and immutable during execution snapshots.
- CEL execution is run with the Ruby GVL released so other Ruby threads can run.
- Ruby callbacks from CEL functions temporarily re-acquire the GVL.

## Development

### Tooling

Uses [mise](https://mise.jdx.dev/) for versions:

```toml
[tools]
ruby = "3.4.8"
rust = "stable"
```

### Test + lint

```bash
bundle exec rake
```

This runs:

- `standard` (format/lint)
- native extension compile
- `rspec`

## Compatibility notes

- Some CEL value variants (e.g. opaque custom Rust types) cannot be fully marshaled to Ruby and raise `CEL::TypeError`.
- This project targets broad CEL compatibility, but parity for every Rust-only extension point is still evolving.
