#include "glimmer_makepad.h"
#include <assert.h>
#include <inttypes.h>
#include <stdio.h>

int main(void) {
    assert(gm_abi_version() == 1);
    uint64_t ctx = gm_context_create();
    uint64_t root = gm_node_create(ctx, GM_VBOX);
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
    }
    assert(gm_context_destroy(ctx) == 0);
    printf("after_destroy rust_live=%" PRIu64 "\n", gm_memory_live_bytes());
    return 0;
}
