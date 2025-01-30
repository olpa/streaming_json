use crate::scan_json::ContextFrame;

pub trait Matcher {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool;
}


pub struct Name {
    name: String,
}

impl Name {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Matcher for Name {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        self.name == name
    }
}


pub struct ParentAndName {
    parent: String,
    name: String,
}

impl ParentAndName {
    pub fn new(parent: String, name: String) -> Self {
        Self { parent, name }
    }
}

impl Matcher for ParentAndName {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        if context.len() == 0 {
            return false;
        }
        self.name == name && context[0].current_key == self.parent
    }
}
