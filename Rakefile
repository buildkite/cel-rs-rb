# frozen_string_literal: true

require "bundler/gem_tasks"
require "rspec/core/rake_task"
require "rb_sys/extensiontask"
require "standard/rake"

RSpec::Core::RakeTask.new(:spec)

GEMSPEC = Gem::Specification.load("cel-rs-rb.gemspec")

RbSys::ExtensionTask.new("cel", GEMSPEC) do |ext|
  ext.lib_dir = "lib/cel"
  ext.ext_dir = "ext/cel"
end

task default: %i[standard compile spec]
task test: :spec
