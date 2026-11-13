# Changelog

## Unreleased

### Fixed

- Release replaced Ruby callbacks and their captured values instead of retaining
  every registration until the `CEL::Context` is collected. In-flight executions
  keep their original callbacks alive and pinned against GC compaction until
  execution finishes, including when execution raises an error or its Ruby thread
  is interrupted or killed.

## 0.3.1

### Fixed

- Keep registered Ruby callback Procs and their captured values alive while their
  `CEL::Context` is reachable. Prevents use-after-free after the caller drops its
  own reference and Ruby runs garbage collection. Callbacks are also pinned during
  compaction, including when collection runs inside a callback.
