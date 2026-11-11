#include "glimmer_makepad.h"
#include <assert.h>
#include <inttypes.h>
#include <stdio.h>

int main(void) {
    assert(gm_abi_version() == GM_ABI_VERSION);
    uint64_t ctx = gm_context_create();
    uint64_t root = gm_node_create(ctx, GM_VBOX);
    uint64_t warm = gm_node_create(ctx, GM_HBOX);
    assert(gm_child_append(ctx, root, warm) == 0);
    assert(gm_child_remove(ctx, root, warm) == 0);
    const uint64_t live = gm_memory_live_bytes();
    const uint64_t allocations = gm_memory_allocation_count();
    for (int round = 0; round < 2; ++round) {
        for (int i = 0; i < 10000; ++i) {
            uint64_t row = gm_node_create(ctx, GM_HBOX);
            assert(gm_child_append(ctx, root, row) == 0);
            assert(gm_child_remove(ctx, root, row) == 0);
        }
        printf("round=%d nodes=%" PRId64 " rust_live=%" PRIu64 " rust_peak=%" PRIu64 " allocations=%" PRIu64 "\n",
               round, gm_context_node_count(ctx), gm_memory_live_bytes(),
               gm_memory_peak_bytes(), gm_memory_allocation_count());
        assert(gm_context_node_count(ctx) == 1);
        assert(gm_memory_live_bytes() == live);
        assert(gm_memory_allocation_count() == allocations);
    }
    assert(gm_context_destroy(ctx) == 0);
    printf("after_destroy rust_live=%" PRIu64 "\n", gm_memory_live_bytes());
    const uint64_t empty = gm_memory_live_bytes();
    for (int i = 0; i < 1000; ++i) {
        ctx = gm_context_create();
        assert(gm_node_create(ctx, GM_LABEL) != 0);
        assert(gm_context_destroy(ctx) == 0);
        assert(gm_memory_live_bytes() == empty);
    }
    return 0;
}
