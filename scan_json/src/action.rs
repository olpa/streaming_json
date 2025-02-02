use crate::matcher::Matcher;
use crate::scan_json::ContextFrame;
use rjiter::RJiter;
use std::cell::RefCell;

#[derive(Debug, PartialEq)]
pub enum StreamOp {
    None,
    ValueIsConsumed,
}

pub type BoxedMatcher = Box<dyn Matcher>;

#[allow(clippy::module_name_repetitions)]
pub type BoxedAction<T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> StreamOp>;

#[allow(clippy::module_name_repetitions)]
pub type BoxedEndAction<T> = Box<dyn Fn(&RefCell<T>)>;

#[derive(Debug)]
pub struct Trigger<BoxedActionT> {
    pub matcher: BoxedMatcher,
    pub action: BoxedActionT,
}

impl<BoxedActionT> Trigger<BoxedActionT> {
    #[must_use]
    pub fn new(matcher: BoxedMatcher, action: BoxedActionT) -> Self {
        Self { matcher, action }
    }
}

pub(crate) fn find_action<'a, BoxedActionT>(
    triggers: &'a [Trigger<BoxedActionT>],
    for_key: &str,
    context: &[ContextFrame],
) -> Option<&'a BoxedActionT> {
    triggers
        .iter()
        .find(|trigger| trigger.matcher.matches(for_key, context))
        .map(|trigger| &trigger.action)
}
