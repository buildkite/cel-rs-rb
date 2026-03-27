# frozen_string_literal: true

require_relative "cel/version"

begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require "cel/#{Regexp.last_match(1)}/cel"
rescue LoadError
  require "cel/cel"
end

module CEL
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
    alias_method :__native_execute, :execute

    def execute(context = nil)
      return __native_execute if context.nil?

      execute_with_context(context)
    end

    def call(context = nil)
      execute(context)
    end
  end
end
