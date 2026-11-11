#ifndef GLIMMER_MAKEPAD_H
#define GLIMMER_MAKEPAD_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Contexts and widget handles belong to the creating OS thread. Zero is invalid.
 * Mutations return 0 on success and a negative status on failure. Handle-returning
 * calls return zero on failure. All failures have a thread-local error message.
 * Context destruction releases all widgets. Removal/replacement releases the old
 * subtree recursively. Reordering preserves identity. No Rust pointers escape.
 */
enum gm_kind { GM_VBOX = 1, GM_HBOX = 2 };
uint32_t gm_abi_version(void);
const char *gm_last_error(void); /* borrowed until the next failing call */
uint64_t gm_context_create(void);
int32_t gm_context_destroy(uint64_t ctx);
uint64_t gm_node_create(uint64_t ctx, uint32_t kind);
int32_t gm_node_destroy(uint64_t ctx, uint64_t node); /* detached subtree only */
int32_t gm_child_append(uint64_t ctx, uint64_t parent, uint64_t child);
int32_t gm_child_remove(uint64_t ctx, uint64_t parent, uint64_t child);
int32_t gm_child_replace(uint64_t ctx, uint64_t parent, uint64_t old, uint64_t replacement);
int32_t gm_child_reorder(uint64_t ctx, uint64_t parent, uint64_t child, uint64_t previous); /* 0 = first */
int64_t gm_child_count(uint64_t ctx, uint64_t parent);
uint64_t gm_child_at(uint64_t ctx, uint64_t parent, size_t index);
int64_t gm_context_node_count(uint64_t ctx);
/* Rust allocator requested bytes only: excludes Chez, GPU and OS allocations. */
uint64_t gm_memory_live_bytes(void);
uint64_t gm_memory_peak_bytes(void);
uint64_t gm_memory_allocation_count(void);
#ifdef __cplusplus
}
#endif
#endif
