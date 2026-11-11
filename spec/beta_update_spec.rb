# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

RSpec.describe "beta:update" do
  it "pins upstream HEAD and prepares increasing gem prereleases without committing or publishing" do
    Dir.mktmpdir("cel-beta-update") do |root|
      upstream = File.join(root, "upstream")
      project = File.join(root, "binding")
      FileUtils.mkdir_p([File.join(upstream, "src"), File.join(project, "script"), File.join(project, "lib/cel"),
        File.join(project, "ext/cel")])
      run = lambda do |*args|
        output, status = Open3.capture2e(*args)
        raise output unless status.success?
        output
      end
      File.write(File.join(upstream, "Cargo.toml"), "[package]\nname = 'cel'\nversion = '0.15.0'\n")
      File.write(File.join(upstream, "src/lib.rs"), "")
      run.call("git", "init", "-q", upstream)
      run.call("git", "-C", upstream, "add", ".")
      run.call("git", "-C", upstream, "-c", "user.name=Test", "-c", "user.email=test@example.com",
        "commit", "-qm", "Initial upstream")
      old_sha = run.call("git", "-C", upstream, "rev-parse", "HEAD").strip
      File.write(File.join(upstream, "src/lib.rs"), "pub fn new_feature() {}\n")
      run.call("git", "-C", upstream, "-c", "user.name=Test", "-c", "user.email=test@example.com",
        "commit", "-qam", "Upstream feature")
      head = run.call("git", "-C", upstream, "rev-parse", "HEAD").strip

      FileUtils.cp(File.expand_path("../script/update-beta.rb", __dir__), File.join(project, "script"))
      File.write(File.join(project, "lib/cel/version.rb"), 'module CEL; VERSION = "0.3.0"; end')
      File.write(File.join(project, "Gemfile"), "gemspec\n")
      File.write(File.join(project, "cel-rs-rb.gemspec"), <<~RUBY)
        require_relative "lib/cel/version"
        Gem::Specification.new do |s|
          s.name = "cel-rs-rb"
          s.version = CEL::VERSION
          s.summary = "Beta updater fixture"
          s.authors = ["Test"]
        end
      RUBY
      manifest_path = File.join(project, "ext/cel/Cargo.toml")
      File.write(manifest_path, <<~TOML)
        [package]
        name = "cel"
        version = "0.1.0"
        [lib]
        path = "../../lib.rs"
        [dependencies.cel]
        git = "https://github.com/cel-rust/cel-rust"
        rev = "#{old_sha}"
      TOML
      File.write(File.join(project, "lib.rs"), "")
      File.write(File.join(project, "Cargo.toml"), "[workspace]\nmembers = ['ext/cel']\n")
      config = File.join(root, "gitconfig")
      File.write(config, "[url \"#{upstream}\"]\n  insteadOf = https://github.com/cel-rust/cel-rust\n")
      env = {"BUNDLE_GEMFILE" => File.join(project, "Gemfile"), "RUBYOPT" => nil,
             "GIT_CONFIG_GLOBAL" => config, "CARGO_NET_GIT_FETCH_WITH_CLI" => "true"}
      updater = [RbConfig.ruby, File.join(project, "script/update-beta.rb")]
      date = Time.now.utc.strftime("%Y%m%d")
      2.times do |index|
        output = run.call(env.merge("BETA_BASE_VERSION" => "0.4.0"), *updater)
        version = "0.4.0.beta.#{date}.#{index + 1}"
        expect(output).to include("Prepared #{version}", head)
        expect(File.read(manifest_path)).to include("rev = \"#{head}\"")
        expect(File.read(File.join(project, "Gemfile.lock"))).to include("cel-rs-rb (#{version})")
        expect(File.read(File.join(project, "Cargo.lock"))).to include("##{head}")
        expect(Gem::Version.new(version)).to be < Gem::Version.new("0.4.0")
      end

      before = File.read(manifest_path)
      output, status = Open3.capture2e(env.merge("BETA_BASE_VERSION" => "0.4"), *updater)
      expect(status.success?).to eq(false)
      expect(output).to include("BETA_BASE_VERSION must be a stable X.Y.Z version")
      expect(File.read(manifest_path)).to eq(before)
    end
  end
end
