# frozen_string_literal: true

require "spec_helper"

RSpec.describe "Source gem packaging" do
  it "includes the checked-in Cargo lockfile" do
    spec = Dir.chdir(File.expand_path("..", __dir__)) do
      Gem::Specification.load("cel-rs-rb.gemspec")
    end

    expect(spec.files).to include("Cargo.lock")
  end
end
