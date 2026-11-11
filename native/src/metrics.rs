use std::{alloc::{GlobalAlloc, Layout, System}, sync::atomic::{AtomicU64, Ordering}};
struct Tracked;
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
fn allocated(size: usize) {
    ALLOCS.fetch_add(1, Ordering::Relaxed);
    let live = LIVE.fetch_add(size as u64, Ordering::Relaxed) + size as u64;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Tracked {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() { allocated(layout.size()); } ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() { allocated(layout.size()); } ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout); LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = System.realloc(ptr, layout, size);
        if !result.is_null() {
            LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed); allocated(size);
        } result
    }
}
#[global_allocator]
static ALLOCATOR: Tracked = Tracked;
/// Metrics cover this Rust library's requested heap bytes, not Chez, GPU or OS allocations.
#[no_mangle]
pub extern "C" fn gm_memory_live_bytes() -> u64 { LIVE.load(Ordering::Relaxed) }
#[no_mangle]
pub extern "C" fn gm_memory_peak_bytes() -> u64 { PEAK.load(Ordering::Relaxed) }
#[no_mangle]
pub extern "C" fn gm_memory_allocation_count() -> u64 { ALLOCS.load(Ordering::Relaxed) }
