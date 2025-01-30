pub trait Matcher {
    fn matches(&self, name: &str, context: &[Context]) -> bool;
}

pub struct Name {
    name: String,
}

impl Matcher for Name {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    fn matches(&self, name: &str, context: &[Context]) -> bool {
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
    fn matches(&self, name: &str, context: &[Context]) -> bool {
        if context.len() == 0 {
            return false;
        }
        self.name == name && context[0].current_key == self.parent
    }
}
