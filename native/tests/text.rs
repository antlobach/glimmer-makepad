use glimmer_makepad::*;
use std::ptr;

#[test]
fn utf8_embedded_nul_and_short_buffers_round_trip() {
    let ctx = gm_context_create();
    let label = gm_node_create(ctx, 3);
    let text = "Glimmer · Ελληνικά · 日本語 · 😀\0tail";
    unsafe {
        assert_eq!(gm_node_set_text(ctx, label, text.as_ptr(), text.len()), 0);
        assert_eq!(gm_node_copy_text(ctx, label, ptr::null_mut(), 0), text.len() as i64);
        let mut short = [0x7f; 3];
        assert_eq!(gm_node_copy_text(ctx, label, short.as_mut_ptr(), 3), -1);
        assert_eq!(short, [0x7f; 3]);
        let mut out = vec![0; text.len()];
        assert_eq!(gm_node_copy_text(ctx, label, out.as_mut_ptr(), out.len()), text.len() as i64);
        assert_eq!(out, text.as_bytes());
        let invalid = [0xff];
        assert!(gm_node_set_text(ctx, label, invalid.as_ptr(), 1) < 0);
        assert!(gm_node_set_text(ctx, label, ptr::null(), 1) < 0);
        assert_eq!(gm_node_copy_text(ctx, label, out.as_mut_ptr(), out.len()), text.len() as i64);
        assert_eq!(out, text.as_bytes());
        assert_eq!(gm_node_set_text(ctx, label, ptr::null(), 0), 0);
        assert_eq!(gm_node_copy_text(ctx, label, ptr::null_mut(), 0), 0);
    }
    assert_eq!(gm_context_destroy(ctx), 0);
}

#[test]
fn leaves_reject_children_and_containers_reject_text() {
    let ctx = gm_context_create();
    let label = gm_node_create(ctx, 3);
    let container = gm_node_create(ctx, 1);
    assert!(gm_child_append(ctx, label, container) < 0);
    unsafe { assert!(gm_node_set_text(ctx, container, b"bad".as_ptr(), 3) < 0); }
    assert_eq!(gm_context_node_count(ctx), 2);
    assert_eq!(gm_context_destroy(ctx), 0);
}
