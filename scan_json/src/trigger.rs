use crate::matcher::Matcher;
use crate::scan_json::ActionResult;
use std::cell::RefCell;
use rjiter::RJiter;
use crate::scan_json::ContextFrame;

pub type TriggerAction<T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> ActionResult>;

pub struct Trigger<T> {
    pub matcher: Matcher,
    pub action: TriggerAction<T>,
}

impl<T> std::fmt::Debug for Trigger<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trigger {{ matcher: {:?}, action: <fn> }}", self.matcher)
    }
}

impl<T> Trigger<T> {
    #[must_use]
    pub fn new(matcher: Matcher, action: TriggerAction<T>) -> Self {
        Self { matcher, action }
    }
}

pub type TriggerEndAction<T> = Box<dyn Fn(&RefCell<T>)>;

pub struct TriggerEnd<T> {
    pub matcher: Matcher,
    pub action: TriggerEndAction<T>,
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
    pub fn new(matcher: Matcher, action: TriggerEndAction<T>) -> Self {
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

impl<T> HasMatcher<TriggerAction<T>> for Trigger<T> {
    fn get_action(&self) -> &TriggerAction<T> {
        &self.action
    }

    fn get_matcher(&self) -> &Matcher {
        &self.matcher
    }
}

impl<T> HasMatcher<TriggerEndAction<T>> for TriggerEnd<T> {
    fn get_action(&self) -> &TriggerEndAction<T> {
        &self.action
    }

    fn get_matcher(&self) -> &Matcher {
        &self.matcher
    }
}

pub(crate) fn find_action<'a, T>(
    triggers: &'a [Trigger<T>],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a TriggerAction<T>> {
    find_trigger_action(triggers, for_key, context)
}

pub(crate) fn find_end_action<'a, T>(
    triggers: &'a [TriggerEnd<T>],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a TriggerEndAction<T>> {
    find_trigger_action(triggers, for_key, context)
}
