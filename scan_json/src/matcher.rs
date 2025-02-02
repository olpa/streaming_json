use crate::scan_json::ContextFrame;

pub trait Matcher: std::fmt::Debug {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool;
}

#[derive(Debug)]
pub struct Name {
    name: String,
}

impl Name {
    #[must_use]
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Matcher for Name {
    fn matches(&self, name: &str, _context: &[ContextFrame]) -> bool {
        self.name == name
    }
}

#[derive(Debug)]
pub struct ParentAndName {
    parent: String,
    name: String,
}

impl ParentAndName {
    #[must_use]
    pub fn new(parent: String, name: String) -> Self {
        Self { parent, name }
    }
}

impl Matcher for ParentAndName {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        if context.is_empty() {
            return false;
        }
        let parent = &context[context.len() - 1];
        self.name == name && parent.current_key == self.parent
    }
}
