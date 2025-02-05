use crate::matcher::Matcher;
use crate::scan::ContextFrame;
use rjiter::RJiter;
use std::cell::RefCell;

#[derive(Debug, PartialEq)]
pub enum StreamOp {
    None,
    ValueIsConsumed,
}

pub type BoxedMatcher<'a> = Box<dyn Matcher + 'a>;

#[allow(clippy::module_name_repetitions)]
pub type BoxedAction<'a, T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> StreamOp + 'a>;

#[allow(clippy::module_name_repetitions)]
pub type BoxedEndAction<'a, T> = Box<dyn Fn(&RefCell<T>) + 'a>;

#[derive(Debug)]
pub struct Trigger<'a, BoxedActionT> {
    pub matcher: BoxedMatcher<'a>,
    pub action: BoxedActionT,
}

impl<'a, BoxedActionT> Trigger<'a, BoxedActionT> {
    #[must_use]
    pub fn new(matcher: BoxedMatcher<'a>, action: BoxedActionT) -> Self {
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
