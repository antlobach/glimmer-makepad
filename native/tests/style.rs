use glimmer_makepad::*;

#[test]
fn invalid_style_does_not_partially_modify_the_widget() {
    let ctx = gm_context_create();
    let label = gm_node_create(ctx, 3);
    let mut original = GmStyle::default();
    original.font_size = 28.0;
    original.color = 0x8b5cf6ff;
    unsafe {
        assert_eq!(gm_node_set_style(ctx, label, &original), 0);
        for invalid in [f64::NAN, f64::INFINITY, -3.0] {
            let mut bad = original;
            bad.width = invalid;
            bad.font_size = 100.0;
            assert!(gm_node_set_style(ctx, label, &bad) < 0);
            let mut actual = GmStyle::default();
            assert_eq!(gm_node_get_style(ctx, label, &mut actual), 0);
            assert_eq!(actual, original);
        }
        let mut bad = original;
        bad.align_x = 1.1;
        assert!(gm_node_set_style(ctx, label, &bad) < 0);
        assert!(gm_node_set_style(ctx, label, std::ptr::null()) < 0);
    }
    assert_eq!(gm_context_destroy(ctx), 0);
}
