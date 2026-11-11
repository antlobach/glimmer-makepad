use std::collections::HashMap;
use crate::next_id;

#[derive(Debug)]
pub struct Error { pub code: i32, pub message: &'static str }
impl Error { pub fn new(code: i32, message: &'static str) -> Self { Self { code, message } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Kind { VBox = 1, HBox = 2 }
impl TryFrom<u32> for Kind {
    type Error = Error;
    fn try_from(n: u32) -> Result<Self, Error> {
        match n { 1 => Ok(Self::VBox), 2 => Ok(Self::HBox), _ => Err(Error::new(-2, "unsupported widget kind")) }
    }
}
pub struct Node { pub kind: Kind, pub parent: Option<u64>, pub children: Vec<u64> }
pub struct Context { pub nodes: HashMap<u64, Node>, pub running: bool }
impl Context {
    pub fn new() -> Self { Self { nodes: HashMap::new(), running: false } }
    pub fn node(&self, id: u64) -> Result<&Node, Error> { self.nodes.get(&id).ok_or_else(|| Error::new(-3, "invalid widget handle")) }
    pub fn create(&mut self, kind: Kind) -> u64 {
        let id = next_id();
        self.nodes.insert(id, Node { kind, parent: None, children: Vec::new() }); id
    }
    fn attachment(&self, parent: u64, child: u64) -> Result<(), Error> {
        self.node(child)?;
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
        Ok(())
    }
    // Iterative destruction keeps deeply nested UI teardown off the call stack.
    fn destroy_tree(&mut self, node: u64) {
        let mut pending = vec![node];
        while let Some(id) = pending.pop() {
            if let Some(node) = self.nodes.remove(&id) { pending.extend(node.children); }
        }
    }
    pub fn destroy_detached(&mut self, node: u64) -> Result<(), Error> {
        if self.node(node)?.parent.is_some() { return Err(Error::new(-4, "destroy requires a detached widget")); }
        self.destroy_tree(node); Ok(())
    }
    pub fn remove(&mut self, parent: u64, child: u64) -> Result<(), Error> {
        let pos = self.position(parent, child)?;
        self.nodes.get_mut(&parent).unwrap().children.remove(pos);
        self.destroy_tree(child); Ok(())
    }
    pub fn replace(&mut self, parent: u64, old: u64, new: u64) -> Result<(), Error> {
        let pos = self.position(parent, old)?;
        self.attachment(parent, new)?;
        self.nodes.get_mut(&parent).unwrap().children[pos] = new;
        self.nodes.get_mut(&new).unwrap().parent = Some(parent);
        self.destroy_tree(old); Ok(())
    }
    pub fn reorder(&mut self, parent: u64, child: u64, previous: u64) -> Result<(), Error> {
        let from = self.position(parent, child)?;
        if previous == child { return Err(Error::new(-4, "a widget cannot precede itself")); }
        let mut to = if previous == 0 { 0 } else { self.position(parent, previous)? + 1 };
        if from < to { to -= 1; }
        let children = &mut self.nodes.get_mut(&parent).unwrap().children;
        children.remove(from); children.insert(to, child); Ok(())
    }
}
