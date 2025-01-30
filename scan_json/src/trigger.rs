use crate::matcher::Matcher;
use crate::scan_json::ActionResult;
use std::cell::RefCell;
use rjiter::RJiter;
use crate::scan_json::ContextFrame;

pub type TriggerAction<T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> ActionResult>;

pub struct Trigger<'a, 'b, T> {
    pub matcher: &'a Matcher,
    pub action: &'b TriggerAction<T>,
}

impl<'a, 'b, T> std::fmt::Debug for Trigger<'a, 'b, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trigger {{ matcher: {:?}, action: <fn> }}", self.matcher)
    }
}

impl<'a, 'b, T> Trigger<'a, 'b, T> {
    #[must_use]
    pub fn new(matcher: &'a Matcher, action: &'b TriggerAction<T>) -> Self {
        Self { matcher, action }
    }
}

pub type TriggerEndAction<T> = Box<dyn Fn(&RefCell<T>)>;

pub struct TriggerEnd<'a, 'b, T> {
    pub matcher: &'a Matcher,
    pub action: &'b TriggerEndAction<T>,
}

impl<'a, 'b, T> std::fmt::Debug for TriggerEnd<'a, 'b, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TriggerEnd {{ matcher: {:?}, action: <fn> }}",
            self.matcher
        )
    }
}

impl<'a, 'b, T> TriggerEnd<'a, 'b, T> {
    #[must_use]
    pub fn new(matcher: &'a Matcher, action: &'b TriggerEndAction<T>) -> Self {
        Self { matcher, action }
    }
}

trait HasMatcher<A> {
    fn get_action(&self) -> &A;
    fn get_matcher(&self) -> &Matcher;
}

fn find_trigger_action<'a, T, A>(
    triggers: &'a [T],
    for_key: &String,
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

impl<'a, 'b, T> HasMatcher<TriggerAction<T>> for Trigger<'a, 'b, T> {
    fn get_action(&self) -> &TriggerAction<T> {
        &self.action
    }

    fn get_matcher(&self) -> &Matcher {
        &self.matcher
    }
}

impl<'a, 'b, T> HasMatcher<TriggerEndAction<T>> for TriggerEnd<'a, 'b, T> {
    fn get_action(&self) -> &TriggerEndAction<T> {
        &self.action
    }

    fn get_matcher(&self) -> &Matcher {
        &self.matcher
    }
}

pub(crate) fn find_action<'a, 'b, 'c, T>(
    triggers: &'c [Trigger<'a, 'b, T>],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a TriggerAction<T>> {
    find_trigger_action(triggers, for_key, context)
}

pub(crate) fn find_end_action<'a, 'b, 'c, T>(
    triggers: &'c [TriggerEnd<'a, 'b, T>],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a TriggerEndAction<T>> {
    find_trigger_action(triggers, for_key, context)
}
