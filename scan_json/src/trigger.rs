use crate::matcher::Matcher;
use crate::scan_json::ActionResult;
use crate::scan_json::ContextFrame;
use rjiter::RJiter;
use std::cell::RefCell;

pub type BoxedMatcher = Box<dyn Matcher>;
pub type BoxedAction<T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> ActionResult>;

pub struct Trigger<T> {
    pub matcher: BoxedMatcher,
    pub action: BoxedAction<T>,
}

impl<T> std::fmt::Debug for Trigger<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trigger {{ matcher: {:?}, action: <fn> }}", self.matcher)
    }
}

impl<T> Trigger<T> {
    #[must_use]
    pub fn new(matcher: BoxedMatcher, action: BoxedAction<T>) -> Self {
        Self { matcher, action }
    }
}

pub type BoxedEndAction<T> = Box<dyn Fn(&RefCell<T>)>;

pub struct TriggerEnd<T> {
    pub matcher: BoxedMatcher,
    pub action: BoxedEndAction<T>,
}

impl<T> std::fmt::Debug for TriggerEnd<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TriggerEnd {{ matcher: {:?}, action: <fn> }}",
            self.matcher
        )
    }
}

impl<T> TriggerEnd<T> {
    #[must_use]
    pub fn new(matcher: BoxedMatcher, action: BoxedEndAction<T>) -> Self {
        Self { matcher, action }
    }
}

trait HasMatcher<A> {
    fn get_matcher(&self) -> &BoxedMatcher;
    fn get_action(&self) -> &A;
}

fn find_trigger_action<'a, T, A>(
    triggers: &'a [T],
    for_key: &str,
    context: &[ContextFrame],
) -> Option<&'a A>
where
    T: HasMatcher<A>,
{
    triggers
        .iter()
        .find(|trigger| trigger.get_matcher().matches(for_key, context))
        .map(HasMatcher::get_action)
}

impl<T> HasMatcher<BoxedAction<T>> for Trigger<T> {
    fn get_matcher(&self) -> &BoxedMatcher {
        &self.matcher
    }

    fn get_action(&self) -> &BoxedAction<T> {
        &self.action
    }
}

impl<T> HasMatcher<BoxedEndAction<T>> for TriggerEnd<T> {
    fn get_matcher(&self) -> &BoxedMatcher {
        &self.matcher
    }

    fn get_action(&self) -> &BoxedEndAction<T> {
        &self.action
    }
}

pub(crate) fn find_action<'a, T>(
    triggers: &'a [Trigger<T>],
    for_key: &str,
    context: &[ContextFrame],
) -> Option<&'a BoxedAction<T>> {
    find_trigger_action(triggers, for_key, context)
}

pub(crate) fn find_end_action<'a, T>(
    triggers: &'a [TriggerEnd<T>],
    for_key: &str,
    context: &[ContextFrame],
) -> Option<&'a BoxedEndAction<T>> {
    find_trigger_action(triggers, for_key, context)
}
