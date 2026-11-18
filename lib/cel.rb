# frozen_string_literal: true

require_relative "cel/version"

begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require "cel/#{Regexp.last_match(1)}/cel"
rescue LoadError
  require "cel/cel"
end

module CEL
  def self.compile(source, extensions: [], max_iterations: 0)
    Program.compile(source, extensions: extensions, max_iterations: max_iterations)
  end

  class Context
    class << self
      alias_method :__native_new, :new

      def new(empty = false)
        __native_new(!!empty)
      end

      def build(empty: false, **variables)
        ctx = new(empty)
        variables.each { |k, v| ctx.add_variable(k.to_s, v) }
        yield(ctx) if block_given?
        ctx
      end
    end

    def define_function(name, &block)
      raise ArgumentError, "block required" unless block

      add_function(name.to_s, block)
    end
  end

  class Program
    class << self
      alias_method :__native_compile, :compile

      def compile(source, extensions: [], max_iterations: 0)
        unless max_iterations.is_a?(Integer) && max_iterations >= 0
          raise ArgumentError, "max_iterations must be a non-negative Integer"
        end

        names = extensions.map(&:to_s)
        raise ArgumentError, "Repeated CEL extension" if names.uniq.length != names.length

        __native_compile(source, names, max_iterations)
      end
    end

    alias_method :__native_execute, :execute

    def execute(context = nil, timeout: nil, interrupt: nil)
      return __native_execute(timeout, interrupt) if context.nil?

      execute_with_context(context, timeout: timeout, interrupt: interrupt)
    end

    alias_method :__native_execute_with_context, :execute_with_context

    def execute_with_context(context, timeout: nil, interrupt: nil)
      __native_execute_with_context(context, timeout, interrupt)
    end

    def call(context = nil, timeout: nil, interrupt: nil)
      execute(context, timeout: timeout, interrupt: interrupt)
    end
  end
end
