use crate::matcher::Matcher;
use crate::scan_json::ActionResult;
use std::cell::RefCell;
use rjiter::RJiter;
use crate::scan_json::ContextFrame;

pub type TriggerAction<T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> ActionResult>;

pub struct Trigger<'m, 'a, T> {
    pub matcher: &'m Matcher,
    pub action: &'a TriggerAction<T>,
}

impl<'m, 'a, T> std::fmt::Debug for Trigger<'m, 'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trigger {{ matcher: {:?}, action: <fn> }}", self.matcher)
    }
}

impl<'m, 'a, T> Trigger<'m, 'a, T> {
    #[must_use]
    pub fn new(matcher: &'m Matcher, action: &'a TriggerAction<T>) -> Self {
        Self { matcher, action }
    }
}

pub type TriggerEndAction<T> = Box<dyn Fn(&RefCell<T>)>;

pub struct TriggerEnd<'m, 'a, T> {
    pub matcher: &'m Matcher,
    pub action: &'a TriggerEndAction<T>,
}

impl<'m, 'a, T> std::fmt::Debug for TriggerEnd<'m, 'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TriggerEnd {{ matcher: {:?}, action: <fn> }}",
            self.matcher
        )
    }
}

impl<'m, 'a, T> TriggerEnd<'m, 'a, T> {
    #[must_use]
    pub fn new(matcher: &'m Matcher, action: &'a TriggerEndAction<T>) -> Self {
        Self { matcher, action }
    }
}

trait HasMatcher<'m, 'a, A> {
    fn get_matcher(&self) -> &'m Matcher;
    fn get_action(&self) -> &'a A;
}

fn find_trigger_action<'a, 't, 'm, T, A>(
    triggers: &'t [T],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a A>
where
    T: HasMatcher<'m, 'a, A>,
{
    triggers
        .iter()
        .find(|trigger| trigger.get_matcher().matches(for_key, context))
        .map(HasMatcher::get_action)
}

impl<'m, 'a, T> HasMatcher<'m, 'a, TriggerAction<T>> for Trigger<'m, 'a, T> {
    fn get_matcher(&self) -> &'m Matcher {
        self.matcher
    }

    fn get_action(&self) -> &'a TriggerAction<T> {
        self.action
    }
}

impl<'m, 'a, T> HasMatcher<'m, 'a, TriggerEndAction<T>> for TriggerEnd<'m, 'a, T> {
    fn get_matcher(&self) -> &'m Matcher {
        self.matcher
    }

    fn get_action(&self) -> &'a TriggerEndAction<T> {
        self.action
    }
}

pub(crate) fn find_action<'m, 'a, 't, T>(
    triggers: &'t [Trigger<'m, 'a, T>],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a TriggerAction<T>> {
    find_trigger_action(triggers, for_key, context)
}

pub(crate) fn find_end_action<'m, 'a, 't, T>(
    triggers: &'t [TriggerEnd<'m, 'a, T>],
    for_key: &String,
    context: &[ContextFrame],
) -> Option<&'a TriggerEndAction<T>> {
    find_trigger_action(triggers, for_key, context)
}
