use glimmer_makepad::*;
use std::{ffi::c_void, sync::atomic::{AtomicUsize, Ordering}, time::Duration};
static WAKE: AtomicUsize = AtomicUsize::new(0);
unsafe extern "C" fn event(ctx: u64, root: u64, event: u32, data: *mut c_void) -> i32 {
    let label = data as usize as u64;
    match event {
        1 => { println!("ready root={root} label={label}"); }
        5 => {
            if WAKE.fetch_add(1, Ordering::SeqCst) == 0 {
                let text = "Updated from the C ABI · Ελληνικά · 日本語 · 😀";
                if gm_node_set_text(ctx, label, text.as_ptr(), text.len()) != 0 { return 1; }
                println!("label updated");
            } else { return gm_close(ctx); }
        }
        4 => { println!("closing nodes={}", gm_context_node_count(ctx)); }
        _ => {}
    }
    0
}
fn main() {
    let ctx = gm_context_create();
    let root = gm_node_create(ctx, 1);
    let label = gm_node_create(ctx, 3);
    let text = "Glimmer Makepad · self-rendered native UI";
    unsafe { assert_eq!(gm_node_set_text(ctx, label, text.as_ptr(), text.len()), 0); }
    assert_eq!(gm_child_append(ctx, root, label), 0);
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(2)); gm_wake();
        std::thread::sleep(Duration::from_secs(4)); gm_wake();
    });
    let title = "Glimmer Makepad — Label verification";
    let result = unsafe { gm_run(ctx, root, title.as_ptr(), title.len(), 780.0, 420.0, Some(event), label as usize as *mut c_void) };
    assert_eq!(result, 0);
    assert_eq!(gm_context_destroy(ctx), 0);
    println!("native loop returned; context destroyed");
}
