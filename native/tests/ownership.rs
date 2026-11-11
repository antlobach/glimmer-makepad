use glimmer_makepad::*;

#[test]
fn lifecycle_reorder_replace_and_recursive_removal() {
    let c = gm_context_create();
    let root = gm_node_create(c, 1);
    let a = gm_node_create(c, 2);
    let b = gm_node_create(c, 1);
    let nested = gm_node_create(c, 1);
    assert_eq!(gm_child_append(c, a, nested), 0);
    assert_eq!(gm_child_append(c, root, a), 0);
    assert_eq!(gm_child_append(c, root, b), 0);
    assert_eq!(gm_child_reorder(c, root, b, 0), 0);
    assert_eq!(gm_child_at(c, root, 0), b);
    assert_eq!(gm_child_at(c, root, 1), a);
    assert_eq!(gm_child_reorder(c, root, b, a), 0);
    assert_eq!(gm_child_at(c, root, 0), a);
    let replacement = gm_node_create(c, 2);
    assert_eq!(gm_child_replace(c, root, a, replacement), 0);
    assert_eq!(gm_child_at(c, root, 0), replacement);
    assert_eq!(gm_child_count(c, a), -1);
    assert_eq!(gm_child_count(c, nested), -1);
    assert_eq!(gm_context_node_count(c), 3);
    assert_eq!(gm_child_remove(c, root, replacement), 0);
    assert_eq!(gm_node_destroy(c, root), 0);
    assert_eq!(gm_context_node_count(c), 0);
    assert_eq!(gm_context_destroy(c), 0);
    assert!(gm_context_destroy(c) < 0);
}

#[test]
fn invalid_mutations_are_atomic_and_handles_are_context_bound() {
    let c = gm_context_create();
    let other = gm_context_create();
    let p = gm_node_create(c, 1);
    let child = gm_node_create(c, 2);
    let foreign = gm_node_create(other, 1);
    assert_eq!(gm_child_append(c, p, child), 0);
    assert!(gm_child_append(c, child, p) < 0);
    assert!(gm_child_append(c, p, child) < 0);
    assert!(gm_child_append(c, p, foreign) < 0);
    assert!(gm_node_destroy(c, child) < 0);
    assert!(gm_child_replace(c, p, child, p) < 0);
    assert!(gm_child_reorder(c, p, child, child) < 0);
    assert!(gm_child_remove(c, child, p) < 0);
    assert_eq!(gm_child_count(c, p), 1);
    assert_eq!(gm_child_at(c, p, 0), child);
    assert_eq!(gm_context_node_count(c), 2);
    assert_eq!(gm_context_destroy(c), 0);
    assert_eq!(gm_context_destroy(other), 0);
}

#[test]
fn foreign_threads_cannot_mutate_contexts() {
    let c = gm_context_create();
    let result = std::thread::spawn(move || gm_node_create(c, 1)).join().unwrap();
    assert_eq!(result, 0);
    assert_eq!(gm_context_node_count(c), 0);
    assert_eq!(gm_context_destroy(c), 0);
}

#[test]
fn repeated_subtree_churn_releases_every_handle() {
    let c = gm_context_create();
    let root = gm_node_create(c, 1);
    for _ in 0..1000 {
        let row = gm_node_create(c, 2);
        let leaf = gm_node_create(c, 1);
        assert_eq!(gm_child_append(c, row, leaf), 0);
        assert_eq!(gm_child_append(c, root, row), 0);
        assert_eq!(gm_child_remove(c, root, row), 0);
        assert_eq!(gm_context_node_count(c), 1);
        assert_eq!(gm_child_count(c, row), -1);
    }
    assert_eq!(gm_context_destroy(c), 0);
}
