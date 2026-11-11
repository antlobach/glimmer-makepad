use super::*;
use std::cell::Cell;

fn setup(kind: Kind) -> (Cx, Context, u64, WindowId, (WindowHandle, Pass, DrawList2d)) {
    let mut cx = Cx::new(Box::new(|_, _| {}));
    makepad_widgets::live_design(&mut cx);
    live_design(&mut cx);
    GmContainer::register_main_module(&mut cx);
    cx.live_expand();
    cx.live_scan_dependencies();
    cx.native_load_dependencies();
    let window = WindowHandle::new(&mut cx);
    let window_id = window.window_id();
    let pass = Pass::new(&mut cx);
    pass.set_size(&mut cx, dvec2(400.0, 200.0));
    window.set_pass(&mut cx, &pass);
    let mut c = Context::new();
    c.running = true;
    let id = c.create(kind);
    c.set_text(id, "Press").unwrap();
    flush(&mut c, &mut cx).unwrap();
    let mut list = DrawList2d::new(&mut cx);
    let draw_event = DrawEvent { redraw_all: true, ..Default::default() };
    {
        let mut draw = CxDraw::new(&mut cx, &draw_event);
        draw.begin_pass(&pass, Some(1.0));
        list.begin_always(&mut draw);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass_sized_turtle(Layout::default());
        c.nodes[&id].widget.as_ref().unwrap().draw_all(&mut cx2d, &mut Scope::empty());
        cx2d.end_pass_sized_turtle();
        list.end(&mut draw);
        draw.end_pass(&pass);
    }
    // Keep the CPU-side draw resources alive while exercising hit testing.
    (cx, c, id, window_id, (window, pass, list))
}

#[test]
fn button_click_requires_a_press_and_release_inside_its_drawn_bounds() {
    let (mut cx, c, id, window, _resources) = setup(Kind::Button);
    let widget = c.nodes[&id].widget.as_ref().unwrap();
    let rect = widget.area().rect(&cx);
    let inside = rect.pos + rect.size * 0.5;
    cx.fingers.first_mouse_button = Some((MouseButton::PRIMARY, window));
    let down = Event::MouseDown(MouseDownEvent {abs: inside, button: MouseButton::PRIMARY, window_id: window, modifiers: Default::default(), handled: Cell::new(Area::Empty), time: 1.0});
    let actions = cx.capture_actions(|cx| widget.handle_event(cx, &down, &mut Scope::empty()));
    assert!(actions.iter().any(|a| matches!(a.downcast_ref::<WidgetAction>().map(|a| a.cast_ref::<ButtonAction>()), Some(ButtonAction::Pressed(_)))));
    let up = Event::MouseUp(MouseUpEvent {abs: inside, button: MouseButton::PRIMARY, window_id: window, modifiers: Default::default(), time: 1.1});
    let actions = cx.capture_actions(|cx| widget.handle_event(cx, &up, &mut Scope::empty()));
    assert_eq!(actions.iter().filter(|a| matches!(a.downcast_ref::<WidgetAction>().map(|a| a.cast_ref::<ButtonAction>()), Some(ButtonAction::Clicked(_)))).count(), 1);
}

#[test]
fn dragging_outside_a_button_does_not_activate_it() {
    let (mut cx, c, id, window, _resources) = setup(Kind::Button);
    let widget = c.nodes[&id].widget.as_ref().unwrap();
    let rect = widget.area().rect(&cx);
    let inside = rect.pos + rect.size * 0.5;
    cx.fingers.first_mouse_button = Some((MouseButton::PRIMARY, window));
    let down = Event::MouseDown(MouseDownEvent {abs: inside, button: MouseButton::PRIMARY, window_id: window, modifiers: Default::default(), handled: Cell::new(Area::Empty), time: 1.0});
    widget.handle_event(&mut cx, &down, &mut Scope::empty());
    let up = Event::MouseUp(MouseUpEvent {abs: rect.pos + rect.size + dvec2(50.0, 50.0), button: MouseButton::PRIMARY, window_id: window, modifiers: Default::default(), time: 1.1});
    let actions = cx.capture_actions(|cx| widget.handle_event(cx, &up, &mut Scope::empty()));
    assert!(actions.iter().any(|a| matches!(a.downcast_ref::<WidgetAction>().map(|a| a.cast_ref::<ButtonAction>()), Some(ButtonAction::Released(_)))));
    assert!(!actions.iter().any(|a| matches!(a.downcast_ref::<WidgetAction>().map(|a| a.cast_ref::<ButtonAction>()), Some(ButtonAction::Clicked(_)))));
}

#[test]
fn input_selection_survives_style_and_identical_controlled_text_updates() {
    use makepad_widgets::text::selection::{Cursor, Selection};
    let (mut cx, mut c, id, _, _resources) = setup(Kind::TextInput);
    let widget = c.nodes[&id].widget.as_ref().unwrap().clone();
    let selected = Selection {cursor: Cursor {index: 3, prefer_next_row: false}, anchor: Cursor {index: 1, prefer_next_row: false}};
    widget.borrow_mut::<TextInput>().unwrap().set_selection(&mut cx, selected);
    c.nodes.get_mut(&id).unwrap().style.font_size = 24.0;
    c.mark(id, 1);
    c.set_text(id, "Press").unwrap();
    flush(&mut c, &mut cx).unwrap();
    let input = widget.borrow::<TextInput>().unwrap();
    assert_eq!(input.selection().cursor, selected.cursor);
    assert_eq!(input.selection().anchor, selected.anchor);
    assert_eq!(input.text(), "Press");
    drop(input);
    c.set_text(id, "replacement").unwrap();
    flush(&mut c, &mut cx).unwrap();
    assert_eq!(widget.text(), "replacement");
}

#[test]
fn text_paints_above_its_own_background_with_differently_colored_siblings() {
    let (mut cx, mut c, first, _, (_window, pass, mut list)) = setup(Kind::Label);
    let root = c.create(Kind::HBox);
    let left = c.create(Kind::VBox);
    let right = c.create(Kind::VBox);
    let second = c.create(Kind::Label);
    c.set_text(second, "Second caption").unwrap();
    c.nodes.get_mut(&left).unwrap().style.background = 0x7255e8ff;
    c.nodes.get_mut(&left).unwrap().style.width = 180.0;
    c.nodes.get_mut(&right).unwrap().style.width = 180.0;
    c.nodes.get_mut(&right).unwrap().style.background = 0x334155ff;
    c.append(root, left).unwrap();
    c.append(root, right).unwrap();
    c.append(left, first).unwrap();
    c.append(right, second).unwrap();
    flush(&mut c, &mut cx).unwrap();
    let event = DrawEvent { redraw_all: true, ..Default::default() };
    {
        let mut draw = CxDraw::new(&mut cx, &event);
        draw.begin_pass(&pass, Some(1.0));
        list.begin_always(&mut draw);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass_sized_turtle(Layout::default());
        c.nodes[&root].widget.as_ref().unwrap().draw_all(&mut cx2d, &mut Scope::empty());
        cx2d.end_pass_sized_turtle();
        list.end(&mut draw);
        draw.end_pass(&pass);
    }
    for (background, caption) in [(left, first), (right, second)] {
        let background = c.nodes[&background].widget.as_ref().unwrap().area();
        let caption = c.nodes[&caption].widget.as_ref().unwrap().area();
        let background = background.valid_instance(&cx).unwrap();
        let caption = caption.valid_instance(&cx).unwrap();
        assert_eq!(caption.draw_list_id, background.draw_list_id);
        assert!(caption.draw_item_id > background.draw_item_id,
            "text was batched underneath its opaque background");
    }
}
