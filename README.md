# Glimmer Makepad

A self-rendered native GUI backend for [Jolt](https://github.com/jolt-lang/jolt)
and [Glimmer](https://github.com/jolt-lang/glimmer), using Makepad through a Rust C ABI.

Glimmer owns application reactivity and reconciliation. Rust owns widget handles,
layout, rendering, input and platform integration. There is no browser, Go layer,
or OS-control wrapper. Makepad is pinned to its published 1.0.0 release.

## Development

Requirements: Rust 1.93+, a platform C toolchain, and Jolt 0.8.25+.

```sh
cargo test --test ownership
cargo build
```

The C contract is in [`include/glimmer_makepad.h`](include/glimmer_makepad.h).
Contexts are confined to their creating OS thread. Widget IDs are opaque and never
reused. Removing or replacing a child destroys its entire subtree; reordering does
not recreate it. Invalid mutations leave the tree unchanged. All C boundaries
contain Rust panics; invalid pointers still violate the caller's C memory contract.

## Memory accounting

`gm_memory_live_bytes`, `gm_memory_peak_bytes`, and `gm_memory_allocation_count`
track allocations made through this library's Rust allocator. They are requested
heap bytes, not resident memory or GPU allocations. `gm_context_node_count` tracks
live widget handles. Context destruction releases the retained tree.

## License

MIT. Makepad and its dependencies retain their own licenses.
