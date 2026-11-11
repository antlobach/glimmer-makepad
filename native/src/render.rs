use makepad_widgets::*;
use std::{cell::RefCell, ffi::c_void, rc::Rc, sync::atomic::{AtomicBool, Ordering}};
use crate::{model::{Context, Error, Kind}, status, utf8, with_context};

live_design! {
    use link::widgets::*;
    use link::theme::*;
    use makepad_draw::shader::std::*;
    GmContainer = {{GmContainer}} {
        width: Fill, height: Fit,
        flow: Down, spacing: 12.0,
        draw_bg: {
            // Preserve painter order across dynamically styled/reordered widgets.
            draw_call_always: true,
            uniform color: #0000,
            uniform border_radius: 0.0,
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                sdf.fill(self.color);
                return sdf.result;
            }
        }
    }
    GmLabel = <Label> { draw_text: { draw_call_always: true, color: #e8eaf0, text_style: {font_size: 20.0} } }
    GmButton = <Button> {
        draw_bg: {draw_call_always: true},
        draw_text: {draw_call_always: true}
    }
    GmTextInput = <TextInput> {
        draw_bg: {draw_call_always: true},
        draw_text: {draw_call_always: true}
    }
    Ui = <Root> {
        main_window = <Window> {
            window: {inner_size: vec2(720, 480), title: "Glimmer Makepad"},
            pass: {clear_color: #111318},
            body = <GmContainer> { width: Fill, height: Fill, padding: 24.0 }
        }
    }
}

// The published Makepad View keeps its child vector private. This dynamic
// container uses Makepad's turtle layout, shaders and event dispatch directly;
// it deliberately does not introduce a second application reconciler.
#[derive(Live, LiveHook, LiveRegisterWidget)]
pub struct GmContainer {
    #[live] draw_bg: DrawQuad,
    #[walk] walk: Walk,
    #[layout] layout: Layout,
    #[rust] children: Vec<(LiveId, WidgetRef)>,
}
impl WidgetNode for GmContainer {
    fn uid_to_widget(&self, uid: WidgetUid) -> WidgetRef {
        for (_, child) in &self.children {
            let found = child.uid_to_widget(uid);
            if !found.is_empty() { return found; }
        }
        WidgetRef::default()
    }
    fn find_widgets(&self, path: &[LiveId], cached: WidgetCache, results: &mut WidgetSet) {
        for (id, child) in &self.children {
            if path.first() == Some(id) {
                if path.len() == 1 { results.push(child.clone()); }
                else { child.find_widgets(&path[1..], cached, results); }
            } else { child.find_widgets(path, cached, results); }
        }
    }
    fn walk(&mut self, _: &mut Cx) -> Walk { self.walk }
    fn area(&self) -> Area { self.draw_bg.area() }
    fn redraw(&mut self, cx: &mut Cx) {
        self.draw_bg.redraw(cx);
        for (_, child) in &self.children { child.redraw(cx); }
    }
}
impl Widget for GmContainer {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        for (_, child) in &self.children { child.handle_event(cx, event, scope); }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.draw_bg.begin(cx, walk, self.layout);
        for (_, child) in &self.children { child.draw_all(cx, scope); }
        self.draw_bg.end(cx);
        DrawStep::done()
    }
}

fn apply_style(cx: &mut Cx, widget: &WidgetRef, kind: Kind, s: &crate::GmStyle) {
    fn size(value: f64) -> LiveValue {
        match value { -1.0 => LiveValue::BareEnum(live_id!(Fill)), -2.0 => LiveValue::BareEnum(live_id!(Fit)), n => LiveValue::Float64(n) }
    }
    let width = size(s.width);
    let height = size(s.height);
    let foreground = Vec4::from_u32(s.color);
    let background = Vec4::from_u32(s.background);
    widget.apply_over(cx, live!{
        width: (width), height: (height), margin: (s.margin), padding: (s.padding),
        align: {x: (s.align_x), y: (s.align_y)}
    });
    if !kind.is_container() {
        widget.apply_over(cx, live!{draw_text: {color: (foreground), text_style: {font_size: (s.font_size)}}});
    }
    if kind != Kind::Label {
        widget.apply_over(cx, live!{spacing: (s.spacing), draw_bg: {color: (background), border_radius: (s.corner_radius)}});
    }
    if kind == Kind::Button {
        widget.apply_over(cx, live!{draw_bg: {color_hover: (background), color_down: (background), color_focus: (background)}});
    }
}

fn template(cx: &mut Cx, name: LiveId) -> Result<WidgetRef, Error> {
    WidgetRef::new_from_module(cx, LiveModuleId::from_str(module_path!()).unwrap(), name)
        .ok_or_else(|| Error::new(-7, "Makepad widget template was not registered"))
}
fn flush(c: &mut Context, cx: &mut Cx) -> Result<(), Error> {
    let mut dirty = std::mem::take(&mut c.dirty);
    for id in &dirty {
        if let Some(node) = c.nodes.get_mut(id) {
            if node.widget.is_none() {
                let widget = template(cx, match node.kind {
                    Kind::Label => live_id!(GmLabel), Kind::Button => live_id!(GmButton),
                    Kind::TextInput => live_id!(GmTextInput),
                    _ => live_id!(GmContainer),
                })?;
                c.by_uid.insert(widget.widget_uid().0, *id);
                node.widget = Some(widget);
            }
        }
    }
    for id in &dirty {
        if let Some(node) = c.nodes.get(id) {
            let widget = node.widget.as_ref().unwrap();
            if node.dirty & 1 != 0 {
                apply_style(cx, widget, node.kind, &node.style);
                match node.kind {
                    Kind::VBox => widget.apply_over(cx, live!{flow: Down}),
                    Kind::HBox => widget.apply_over(cx, live!{flow: Right}),
                    _ => {}
                }
            }
            if node.dirty & 4 != 0 && !node.kind.is_container() {
                widget.set_text(cx, &node.text);
            }
            if node.dirty & 8 != 0 && node.kind == Kind::TextInput {
                let placeholder = &node.placeholder;
                widget.apply_over(cx, live!{empty_text: (placeholder)});
            }
            if node.dirty & 2 != 0 && node.kind.is_container() {
                let mut container = widget.borrow_mut::<GmContainer>().ok_or_else(|| Error::new(-7, "invalid Makepad container"))?;
                container.children.clear();
                for child in &node.children {
                    container.children.push((LiveId(*child), c.node(*child)?.widget.as_ref().unwrap().clone()));
                }
            }
        }
    }
    for id in &dirty { if let Some(node) = c.nodes.get_mut(id) { node.dirty = 0; } }
    if !dirty.is_empty() { cx.redraw_all(); }
    dirty.clear();
    c.dirty = dirty;
    Ok(())
}

pub type Callback = unsafe extern "C" fn(u64, u64, u32, *mut c_void) -> i32;
static LOOP_RUNNING: AtomicBool = AtomicBool::new(false);
struct Runner { ui: WidgetRef, started: bool, closed: bool }
struct ContextRunGuard(u64);
impl Drop for ContextRunGuard {
    fn drop(&mut self) {
        let _ = with_context(self.0, |c| {
            c.running = false;
            c.root = None;
            c.dirty.clear();
            c.by_uid.clear();
            for node in c.nodes.values_mut() { node.widget = None; node.dirty = 15; }
            Ok(())
        });
    }
}

/// Runs a process-owning native application. After startup, Makepad shutdown
/// terminates the process; this function does not return to its host.
/// Callbacks run on the UI thread and must contain foreign exceptions.
/// Events: ready=1, click=2, change=3, closing=4, wake=5, submit=6.
/// # Safety
/// `title` is a readable UTF-8 span. The callback and user data must remain valid
/// until this function returns; the callback must not unwind across C frames.
#[no_mangle]
pub unsafe extern "C" fn gm_run(ctx: u64, root: u64, title: *const u8, title_len: usize, width: f64, height: f64, callback: Option<Callback>, data: *mut c_void) -> i32 {
    status(|| {
        let title = utf8(title, title_len)?.to_owned();
        if title.contains('\0') || !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 || width > f32::MAX as f64 || height > f32::MAX as f64 {
            return Err(Error::new(-2, "invalid window title or size"));
        }
        #[cfg(target_os = "macos")]
        {
            extern "C" { fn pthread_main_np() -> i32; }
            if pthread_main_np() == 0 { return Err(Error::new(-6, "the native GUI must run on the OS main thread")); }
        }
        with_context(ctx, |c| {
            let node = c.node(root)?;
            if c.running || node.parent.is_some() || !node.kind.is_container() { return Err(Error::new(-6, "run requires an idle context and detached container root")); }
            Ok(())
        })?;
        static PRESTART: std::sync::LazyLock<bool> = std::sync::LazyLock::new(Cx::pre_start);
        if *PRESTART {
            return Err(Error::new(-6, "Makepad selected an auxiliary process mode"));
        }
        if LOOP_RUNNING.swap(true, Ordering::SeqCst) {
            return Err(Error::new(-6, "the native application loop is already running"));
        }
        struct LoopGuard;
        impl Drop for LoopGuard { fn drop(&mut self) { LOOP_RUNNING.store(false, Ordering::SeqCst); } }
        let _guard = LoopGuard;
        with_context(ctx, |c| { c.running = true; c.root = Some(root); c.close_requested = false; c.dirty.clear(); c.dirty.extend(c.nodes.keys().copied()); Ok(()) })?;
        let _context_guard = ContextRunGuard(ctx);
        let runner = Rc::new(RefCell::new(Runner { ui: WidgetRef::default(), started: false, closed: false }));
        let shared = runner.clone();
        let notify = move |node, event| -> Result<(), Error> {
            if let Some(callback) = callback {
                if callback(ctx, node, event, data) != 0 { return Err(Error::new(-8, "application event callback failed")); }
            }
            Ok(())
        };
        let cx = Rc::new(RefCell::new(Cx::new(Box::new(move |cx, event| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), Error> {
                let mut runner = shared.borrow_mut();
                if matches!(event, Event::Startup) {
                    runner.ui = template(cx, live_id!(Ui))?;
                    let size = vec2(width as f32, height as f32);
                    runner.ui.widget(id!(main_window)).apply_over(cx, live!{window: {title: (title), inner_size: (size)}});
                    with_context(ctx, |c| flush(c, cx))?;
                    let root_widget = with_context(ctx, |c| Ok(c.node(root)?.widget.as_ref().unwrap().clone()))?;
                    let body = runner.ui.widget(id!(main_window.body));
                    body.borrow_mut::<GmContainer>().ok_or_else(|| Error::new(-7, "missing root container"))?.children.push((LiveId(root), root_widget));
                    runner.started = true;
                    notify(root, 1)?;
                }
                if !runner.closed && matches!(event, Event::Signal) { notify(0, 5)?; }
                if let Event::Actions(actions) = event {
                    let pending = with_context(ctx, |c| {
                        let mut pending = Vec::new();
                        for action in actions.iter().filter_map(|a| a.downcast_ref::<WidgetAction>()) {
                            if let Some(&id) = c.by_uid.get(&action.widget_uid.0) {
                                if matches!(action.cast_ref::<ButtonAction>(), ButtonAction::Clicked(_)) {
                                    pending.push((id, 2));
                                }
                                match action.cast_ref::<TextInputAction>() {
                                    TextInputAction::Changed(text) => {
                                        let node = c.nodes.get_mut(&id).unwrap();
                                        node.text.clear();
                                        node.text.push_str(text);
                                        pending.push((id, 3));
                                    }
                                    TextInputAction::Returned(_, _) => pending.push((id, 6)),
                                    _ => {}
                                }
                            }
                        }
                        Ok(pending)
                    })?;
                    for (id, event) in pending {
                        if with_context(ctx, |c| Ok(c.nodes.contains_key(&id)))? { notify(id, event)?; }
                    }
                }
                let closing = matches!(event, Event::WindowCloseRequested(_) | Event::WindowClosed(_) | Event::Shutdown)
                    || with_context(ctx, |c| Ok(c.close_requested))?;
                if closing && !runner.closed {
                    runner.closed = true;
                    notify(root, 4)?;
                    with_context(ctx, |c| flush(c, cx))?;
                    cx.quit();
                }
                // Stock Makepad can terminate the process itself. Do the same
                // when it dispatches Shutdown, including timer-driven quit.
                // No host finally blocks or post-run cleanup are promised.
                if matches!(event, Event::Shutdown) { std::process::exit(0); }
                if runner.started && !runner.closed {
                    with_context(ctx, |c| flush(c, cx))?;
                    runner.ui.handle_event(cx, event, &mut Scope::empty());
                }
                Ok(())
            })).unwrap_or_else(|_| Err(Error::new(-7, "Rust panic contained in the native event handler")));
            if let Err(error) = result {
                eprintln!("Glimmer Makepad application failed: {}", error.message);
                std::process::exit(1);
            }
        }))));
        makepad_widgets::live_design(&mut cx.borrow_mut());
        live_design(&mut cx.borrow_mut());
        GmContainer::register_main_module(&mut cx.borrow_mut());
        cx.borrow_mut().init_cx_os();
        Cx::event_loop(cx.clone());
        // Platforms that return instead of terminating still own this process.
        // Deliver teardown before exit; never resume the host after startup.
        let mut runner = runner.borrow_mut();
        if runner.started && !runner.closed {
            runner.closed = true;
            if let Err(error) = notify(root, 4) {
                eprintln!("Glimmer Makepad application failed: {}", error.message);
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    })
}
#[no_mangle]
pub extern "C" fn gm_close(ctx: u64) -> i32 { status(|| with_context(ctx, |c| { if !c.running { return Err(Error::new(-6, "context is not running")); } c.close_requested = true; Ok(()) })) }
/// The only mutation permitted from any thread; wakes the application's callback.
#[no_mangle]
pub extern "C" fn gm_wake() { SignalToUI::set_ui_signal(); }

#[cfg(test)]
mod tests;
