use std::collections::HashMap;
use crate::next_id;

#[derive(Debug)]
pub struct Error { pub code: i32, pub message: &'static str }
impl Error { pub fn new(code: i32, message: &'static str) -> Self { Self { code, message } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Kind { VBox = 1, HBox = 2, Label = 3, Button = 4, TextInput = 5 }
impl TryFrom<u32> for Kind {
    type Error = Error;
    fn try_from(n: u32) -> Result<Self, Error> {
        match n { 1 => Ok(Self::VBox), 2 => Ok(Self::HBox), 3 => Ok(Self::Label), 4 => Ok(Self::Button), 5 => Ok(Self::TextInput), _ => Err(Error::new(-2, "unsupported widget kind")) }
    }
}
impl Kind {
    pub fn is_container(self) -> bool { matches!(self, Self::VBox | Self::HBox) }
}
pub struct Node {
    pub kind: Kind,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub text: String,
    pub placeholder: String,
    pub widget: Option<makepad_widgets::WidgetRef>,
    pub dirty: u8,
    pub style: crate::GmStyle,
}
pub struct Context {
    pub nodes: HashMap<u64, Node>,
    pub running: bool,
    pub by_uid: HashMap<u64, u64>,
    pub dirty: Vec<u64>,
    pub root: Option<u64>,
    pub close_requested: bool,
}
impl Context {
    pub fn new() -> Self { Self { nodes: HashMap::new(), running: false, dirty: Vec::new(), close_requested: false, by_uid: HashMap::new(), root: None } }
    pub fn node(&self, id: u64) -> Result<&Node, Error> { self.nodes.get(&id).ok_or_else(|| Error::new(-3, "invalid widget handle")) }
    pub fn create(&mut self, kind: Kind) -> u64 {
        let id = next_id();
        self.nodes.insert(id, Node { kind, parent: None, children: Vec::new(), text: String::new(), placeholder: String::new(), widget: None, dirty: 15, style: crate::GmStyle::default() });
        if self.running { self.dirty.push(id); }
        id
    }
    pub fn mark(&mut self, id: u64, flags: u8) {
        let node = self.nodes.get_mut(&id).unwrap();
        if node.dirty == 0 && self.running { self.dirty.push(id); }
        node.dirty |= flags;
    }
    pub fn set_text(&mut self, id: u64, text: &str) -> Result<(), Error> {
        let node = self.nodes.get_mut(&id).ok_or_else(|| Error::new(-3, "invalid widget handle"))?;
        if node.kind.is_container() { return Err(Error::new(-5, "widget does not accept text")); }
        if node.text != text {
            node.text.clear();
            node.text.push_str(text);
            self.mark(id, 4);
        }
        Ok(())
    }
    fn attachment(&self, parent: u64, child: u64) -> Result<(), Error> {
        if self.root == Some(child) { return Err(Error::new(-4, "cannot reparent the running application root")); }
        self.node(child)?;
        if !self.node(parent)?.kind.is_container() { return Err(Error::new(-5, "leaf widgets cannot contain children")); }
        let mut cursor = Some(parent);
        while let Some(id) = cursor {
            if id == child { return Err(Error::new(-4, "attachment would create a cycle")); }
            cursor = self.node(id)?.parent;
        }
        if self.node(child)?.parent.is_some() { return Err(Error::new(-4, "child is already attached")); }
        Ok(())
    }
    fn position(&self, parent: u64, child: u64) -> Result<usize, Error> {
        self.node(child)?;
        self.node(parent)?.children.iter().position(|&n| n == child).ok_or_else(|| Error::new(-4, "widget is not a child of this parent"))
    }
    pub fn append(&mut self, parent: u64, child: u64) -> Result<(), Error> {
        self.attachment(parent, child)?;
        self.nodes.get_mut(&parent).unwrap().children.push(child);
        self.nodes.get_mut(&child).unwrap().parent = Some(parent);
        self.mark(parent, 2);
        Ok(())
    }
    // Iterative destruction keeps deeply nested UI teardown off the call stack.
    fn take_node(&mut self, id: u64) -> Option<Node> {
        let node = self.nodes.remove(&id)?;
        if let Some(widget) = &node.widget { self.by_uid.remove(&widget.widget_uid().0); }
        Some(node)
    }
    fn destroy_tree(&mut self, node: u64) {
        let mut pending = match self.take_node(node) {
            Some(node) => node.children,
            None => return,
        };
        while let Some(id) = pending.pop() {
            if let Some(node) = self.take_node(id) { pending.extend(node.children); }
        }
    }
    pub fn destroy_detached(&mut self, node: u64) -> Result<(), Error> {
        if self.node(node)?.parent.is_some() { return Err(Error::new(-4, "destroy requires a detached widget")); }
        if self.root == Some(node) { return Err(Error::new(-4, "cannot destroy the running application root")); }
        self.destroy_tree(node); Ok(())
    }
    pub fn remove(&mut self, parent: u64, child: u64) -> Result<(), Error> {
        let pos = self.position(parent, child)?;
        self.nodes.get_mut(&parent).unwrap().children.remove(pos);
        self.mark(parent, 2);
        self.destroy_tree(child); Ok(())
    }
    pub fn replace(&mut self, parent: u64, old: u64, new: u64) -> Result<(), Error> {
        let pos = self.position(parent, old)?;
        self.attachment(parent, new)?;
        self.nodes.get_mut(&parent).unwrap().children[pos] = new;
        self.nodes.get_mut(&new).unwrap().parent = Some(parent);
        self.mark(parent, 2);
        self.destroy_tree(old); Ok(())
    }
    pub fn reorder(&mut self, parent: u64, child: u64, previous: u64) -> Result<(), Error> {
        let from = self.position(parent, child)?;
        if previous == child { return Err(Error::new(-4, "a widget cannot precede itself")); }
        let mut to = if previous == 0 { 0 } else { self.position(parent, previous)? + 1 };
        if from < to { to -= 1; }
        let children = &mut self.nodes.get_mut(&parent).unwrap().children;
        children.remove(from); children.insert(to, child);
        self.mark(parent, 2);
        Ok(())
    }
}
