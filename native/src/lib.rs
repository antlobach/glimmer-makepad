//! Retained widget ownership and a C ABI for Glimmer. See include/glimmer_makepad.h.
mod model;
mod metrics;

use model::{Context, Error, Kind};
use std::{cell::RefCell, collections::HashMap, ffi::{c_char, CString}, panic::{catch_unwind, AssertUnwindSafe}, sync::atomic::{AtomicU64, Ordering}};

pub(crate) static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub(crate) fn next_id() -> u64 { NEXT_ID.fetch_add(1, Ordering::Relaxed) }
thread_local! {
    pub(crate) static CONTEXTS: RefCell<HashMap<u64, Context>> = RefCell::new(HashMap::new());
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("").unwrap());
}

pub(crate) fn fail(error: Error) -> i32 {
    LAST_ERROR.with(|s| *s.borrow_mut() = CString::new(error.message).unwrap());
    error.code
}
pub(crate) fn boundary<T: Copy>(fallback: T, f: impl FnOnce() -> Result<T, Error>) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => { fail(error); fallback }
        Err(_) => { fail(Error::new(-7, "Rust panic contained at the C ABI")); fallback }
    }
}
pub(crate) fn status(f: impl FnOnce() -> Result<(), Error>) -> i32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => fail(error),
        Err(_) => fail(Error::new(-7, "Rust panic contained at the C ABI")),
    }
}
pub(crate) fn with_context<T>(id: u64, f: impl FnOnce(&mut Context) -> Result<T, Error>) -> Result<T, Error> {
    CONTEXTS.with(|all| {
        let mut all = all.try_borrow_mut().map_err(|_| Error::new(-6, "reentrant context access"))?;
        let ctx = all.get_mut(&id).ok_or_else(|| Error::new(-1, "invalid context or wrong calling thread"))?;
        f(ctx)
    })
}
/// # Safety
/// Non-null `data` must point to `len` readable bytes for the duration of the call.
pub(crate) unsafe fn utf8<'a>(data: *const u8, len: usize) -> Result<&'a str, Error> {
    if len == 0 { return Ok(""); }
    if data.is_null() || len > isize::MAX as usize { return Err(Error::new(-2, "invalid byte span")); }
    std::str::from_utf8(std::slice::from_raw_parts(data, len)).map_err(|_| Error::new(-2, "text is not UTF-8"))
}

#[no_mangle]
pub extern "C" fn gm_abi_version() -> u32 { 1 }
/// Borrowed thread-local error string. Valid until the next failing call on this thread.
#[no_mangle]
pub extern "C" fn gm_last_error() -> *const c_char { LAST_ERROR.with(|s| s.borrow().as_ptr()) }
#[no_mangle]
pub extern "C" fn gm_context_create() -> u64 {
    boundary(0, || { let id = next_id(); CONTEXTS.with(|all| all.borrow_mut().insert(id, Context::new())); Ok(id) })
}
#[no_mangle]
pub extern "C" fn gm_context_destroy(ctx: u64) -> i32 {
    status(|| CONTEXTS.with(|all| {
        let mut all = all.borrow_mut();
        let context = all.get(&ctx).ok_or_else(|| Error::new(-1, "invalid context or wrong calling thread"))?;
        if context.running { return Err(Error::new(-6, "cannot destroy a running context")); }
        all.remove(&ctx);
        Ok(())
    }))
}
#[no_mangle]
pub extern "C" fn gm_node_create(ctx: u64, kind: u32) -> u64 {
    boundary(0, || with_context(ctx, |c| { let kind = Kind::try_from(kind)?; Ok(c.create(kind)) }))
}
#[no_mangle]
pub extern "C" fn gm_node_destroy(ctx: u64, node: u64) -> i32 { status(|| with_context(ctx, |c| c.destroy_detached(node))) }
#[no_mangle]
pub extern "C" fn gm_child_append(ctx: u64, parent: u64, child: u64) -> i32 { status(|| with_context(ctx, |c| c.append(parent, child))) }
#[no_mangle]
pub extern "C" fn gm_child_remove(ctx: u64, parent: u64, child: u64) -> i32 { status(|| with_context(ctx, |c| c.remove(parent, child))) }
#[no_mangle]
pub extern "C" fn gm_child_replace(ctx: u64, parent: u64, old: u64, new: u64) -> i32 { status(|| with_context(ctx, |c| c.replace(parent, old, new))) }
#[no_mangle]
pub extern "C" fn gm_child_reorder(ctx: u64, parent: u64, child: u64, previous: u64) -> i32 { status(|| with_context(ctx, |c| c.reorder(parent, child, previous))) }
#[no_mangle]
pub extern "C" fn gm_child_count(ctx: u64, parent: u64) -> i64 { boundary(-1, || with_context(ctx, |c| Ok(c.node(parent)?.children.len() as i64))) }
#[no_mangle]
pub extern "C" fn gm_child_at(ctx: u64, parent: u64, index: usize) -> u64 {
    boundary(0, || with_context(ctx, |c| c.node(parent)?.children.get(index).copied().ok_or_else(|| Error::new(-2, "child index out of range"))))
}
#[no_mangle]
pub extern "C" fn gm_context_node_count(ctx: u64) -> i64 { boundary(-1, || with_context(ctx, |c| Ok(c.nodes.len() as i64))) }
