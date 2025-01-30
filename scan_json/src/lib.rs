pub mod matcher;
pub mod scan_json;
pub mod trigger;

pub use scan_json::{scan_json, ActionResult, ContextFrame};
pub use matcher::{Matcher, Name, ParentAndName};
pub use trigger::{Trigger, TriggerAction, TriggerEnd, TriggerEndAction};
