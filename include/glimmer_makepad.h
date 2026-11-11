#ifndef GLIMMER_MAKEPAD_H
#define GLIMMER_MAKEPAD_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Contexts/widgets belong to the creating OS thread; IDs are never reused.
 * Zero is an invalid handle. Mutations return 0 or a negative gm_status.
 * Removal/replacement destroys the old subtree; reorder preserves identity.
 * Byte spans are UTF-8, may contain NUL, and are copied during the call.
 * Pointers must be valid, correctly aligned C storage for their declared size.
 * Only gm_wake and the global memory counters may be called from any thread.
 */
enum gm_kind { GM_VBOX = 1, GM_HBOX = 2, GM_LABEL = 3, GM_BUTTON = 4, GM_TEXT_INPUT = 5 };
enum gm_status {
    GM_OK = 0, GM_INVALID_CONTEXT = -1, GM_INVALID_ARGUMENT = -2,
    GM_INVALID_WIDGET = -3, GM_OWNERSHIP_ERROR = -4, GM_WRONG_KIND = -5,
    GM_LIFECYCLE_ERROR = -6, GM_PANIC = -7, GM_CALLBACK_ERROR = -8
};
enum gm_event { GM_READY = 1, GM_CLICK = 2, GM_CHANGE = 3, GM_CLOSING = 4, GM_WAKE = 5, GM_SUBMIT = 6 };
/* Return 0 on success; contain all foreign exceptions and return nonzero on failure.
 * The callback may call widget APIs. GM_READY/GM_CLOSING carry the root widget;
 * GM_CLICK/GM_CHANGE/GM_SUBMIT carry the control; GM_WAKE carries zero.
 * gm_node_copy_text reads the current input value during GM_CHANGE/GM_SUBMIT.
 */
typedef int32_t (*gm_callback)(uint64_t ctx, uint64_t node, uint32_t event, void *data);
typedef struct gm_style {
    double width, height; /* -1 = fill, -2 = fit, otherwise nonnegative logical pixels */
    double spacing, padding, margin, font_size, corner_radius, align_x, align_y;
    uint32_t color, background; /* numeric 0xRRGGBBAA, independent of byte order */
} gm_style;
uint32_t gm_abi_version(void);
const char *gm_last_error(void); /* borrowed until the next failing call on this thread */
uint64_t gm_context_create(void);
int32_t gm_context_destroy(uint64_t ctx); /* running contexts cannot be destroyed */
uint64_t gm_node_create(uint64_t ctx, uint32_t kind);
int32_t gm_node_destroy(uint64_t ctx, uint64_t node); /* detached subtree only; not a running root */
int32_t gm_child_append(uint64_t ctx, uint64_t parent, uint64_t child);
int32_t gm_child_remove(uint64_t ctx, uint64_t parent, uint64_t child);
int32_t gm_child_replace(uint64_t ctx, uint64_t parent, uint64_t old, uint64_t replacement);
int32_t gm_child_reorder(uint64_t ctx, uint64_t parent, uint64_t child, uint64_t previous); /* 0 = first */
int64_t gm_child_count(uint64_t ctx, uint64_t parent); /* -1 on error */
uint64_t gm_child_at(uint64_t ctx, uint64_t parent, size_t index); /* 0 on error */
int64_t gm_context_node_count(uint64_t ctx); /* -1 on error */
int32_t gm_node_set_text(uint64_t ctx, uint64_t node, const uint8_t *data, size_t len);
/* Returns UTF-8 length, or -1. NULL/0 queries the size. Short buffers fail without
 * writing. No trailing NUL is added. The UI thread serializes edits and reads. */
int64_t gm_node_copy_text(uint64_t ctx, uint64_t node, uint8_t *buffer, size_t capacity);
int32_t gm_node_set_placeholder(uint64_t ctx, uint64_t node, const uint8_t *data, size_t len);
int32_t gm_node_set_style(uint64_t ctx, uint64_t node, const gm_style *style); /* atomic validation */
int32_t gm_node_get_style(uint64_t ctx, uint64_t node, gm_style *style);
/* One native application loop per process, on its OS main thread. Reload the root
 * within the running loop. Callback/data stay valid until return. Desktop entry
 * point: mobile activity/application bootstrapping is not supplied by this API. */
int32_t gm_run(uint64_t ctx, uint64_t root, const uint8_t *title, size_t title_len,
               double width, double height, gm_callback callback, void *data);
int32_t gm_close(uint64_t ctx);
void gm_wake(void);
/* This library's Rust requested heap bytes: excludes Chez, GPU and OS memory.
 * Small process-lifetime toolkit caches and registry capacities can remain. */
uint64_t gm_memory_live_bytes(void);
uint64_t gm_memory_peak_bytes(void);
uint64_t gm_memory_allocation_count(void);
#ifdef __cplusplus
}
#endif
#endif
