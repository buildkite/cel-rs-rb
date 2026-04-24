# frozen_string_literal: true

require_relative "lib/cel/version"

Gem::Specification.new do |spec|
  spec.name = "cel-rs-rb"
  spec.version = CEL::VERSION
  spec.authors = ["CEL Ruby Contributors"]
  spec.email = []

  spec.summary = "Ruby bindings for the Rust CEL crate"
  spec.description = "Robust Ruby bindings to the Rust CEL implementation using Magnus"
  spec.homepage = "https://github.com/catkins/cel-rs-rb"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/catkins/cel-rs-rb"
  spec.metadata["rubygems_mfa_required"] = "true"

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "spec/**/*.rb",
    "Cargo.toml",
    "LICENSE",
    "README.md"
  ]

  spec.bindir = "bin"
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/cel/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9"

  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
  spec.add_development_dependency "rspec", "~> 3.12"
  spec.add_development_dependency "standard", "~> 1.45"
end
