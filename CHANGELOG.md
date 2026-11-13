# Changelog

## 0.3.1

### Fixed

- Keep registered Ruby callback Procs and their captured values alive while their
  `CEL::Context` is reachable. Prevents use-after-free after the caller drops its
  own reference and Ruby runs garbage collection. Callbacks are also pinned during
  compaction, including when collection runs inside a callback.
