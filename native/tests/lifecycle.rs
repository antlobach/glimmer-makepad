use glimmer_makepad::*;

#[test]
fn invalid_launch_returns_without_taking_process_ownership() {
    let ctx = gm_context_create();
    let root = gm_node_create(ctx, 1);
    let invalid: &[(&[u8], f64, f64)] = &[
        (b"Window", 0.0, 480.0),
        (b"Window", 640.0, -1.0),
        (b"Window", f64::NAN, 480.0),
        (b"Window", 640.0, f64::INFINITY),
        (b"Embedded\0NUL", 640.0, 480.0),
        (&[0xff], 640.0, 480.0),
    ];
    for &(title, width, height) in invalid {
        let status = unsafe {
            gm_run(ctx, root, title.as_ptr(), title.len(), width, height, None, std::ptr::null_mut())
        };
        assert_eq!(status, -2);
        assert_eq!(gm_context_node_count(ctx), 1);
        assert_eq!(gm_close(ctx), -6);
    }
    // Failed startup leaves the caller's context usable and caller-owned.
    let label = gm_node_create(ctx, 3);
    assert_ne!(label, 0);
    assert_eq!(gm_child_append(ctx, root, label), 0);
    assert_eq!(gm_context_destroy(ctx), 0);
}
