# frozen_string_literal: true

require "json"
require "open3"

Dir.chdir(File.expand_path("..", __dir__)) do
  version_file = "lib/cel/version.rb"
  version_source = File.read(version_file)
  current = version_source.match(/VERSION = "([^"]+)"/).captures.first
  base = ENV.fetch("BETA_BASE_VERSION", current.split(".").first(3).join("."))
  abort "BETA_BASE_VERSION must be a stable X.Y.Z version" unless base.match?(/\A\d+\.\d+\.\d+\z/)

  date = Time.now.utc.strftime("%Y%m%d")
  previous = current.match(/\A#{Regexp.escape(base)}\.beta\.#{date}\.(\d+)\z/)
  version = "#{base}.beta.#{date}.#{previous ? previous[1].to_i + 1 : 1}"
  upstream = "https://github.com/cel-rust/cel-rust"
  output, status = Open3.capture2("git", "ls-remote", upstream, "HEAD")
  abort "Could not resolve upstream's default branch" unless status.success?
  revision = output.split.first
  abort "Expected a full upstream commit SHA" unless revision&.match?(/\A[0-9a-f]{40}\z/)

  manifest_file = "ext/cel/Cargo.toml"
  manifest = File.read(manifest_file)
  abort "Expected an existing pinned cel Git dependency" unless manifest.match?(/^rev = "[0-9a-f]{40}"$/)
  File.write(manifest_file, manifest.sub(/^rev = "[0-9a-f]{40}"$/, "rev = \"#{revision}\""))
  File.write(version_file, version_source.sub(/VERSION = "[^"]+"/, "VERSION = \"#{version}\""))

  output, status = Open3.capture2("cargo", "metadata", "--format-version", "1")
  abort "Cargo resolution failed; inspect the working diff before retrying" unless status.success?
  dependency = JSON.parse(output).fetch("packages").find do |package|
    package["name"] == "cel" && package["source"]&.start_with?("git+#{upstream}?")
  end
  abort "Cargo did not resolve the requested upstream SHA" unless dependency&.fetch("source")&.end_with?("##{revision}")
  abort "Bundler lock update failed" unless system("bundle", "lock", "--local")

  puts "Prepared #{version} using upstream cel #{dependency.fetch("version")} at #{revision}"
  puts "Review the diff, run mise run test, and open an update PR. No commit, push, tag, or publication was performed."
end
