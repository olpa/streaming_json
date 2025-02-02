use scan_json::matcher::{Matcher, Name};
use scan_json::ContextFrame;

#[test]
fn test_match_by_name() {
    let matcher = Name::new("foo".to_string());
    let context = [];

    assert!(matcher.matches("foo", &context), "Should match exact name");
    assert!(
        !matcher.matches("bar", &context),
        "Should not match different name"
    );
}

#[test]
fn test_match_by_parent_and_name() {
    use scan_json::matcher::ParentAndName;

    let matcher = ParentAndName::new("parent".to_string(), "child".to_string());
    
    let empty_context = [];
    assert!(!matcher.matches("child", &empty_context), "Should not match without context");

    let wrong_parent_context = [ContextFrame::new("wrong".to_string())];
    assert!(!matcher.matches("child", &wrong_parent_context), "Should not match with wrong parent");

    let wrong_name_context = [ContextFrame::new("parent".to_string())];
    assert!(!matcher.matches("wrong", &wrong_name_context), "Should not match with wrong name");

    let matching_context = [ContextFrame::new("parent".to_string())];
    assert!(matcher.matches("child", &matching_context), "Should match with correct parent and name");
}

#[test]
fn test_match_by_parent_and_name_long_context() {
    use scan_json::matcher::ParentAndName;

    let matcher = ParentAndName::new("parent".to_string(), "child".to_string());
    
    let long_context = [
        ContextFrame::new("grandparent".to_string()),
        ContextFrame::new("parent".to_string()),
        ContextFrame::new("child".to_string())
    ];

    assert!(
        !matcher.matches("child", &long_context),
        "Should not match when parent is deeper in context"
    );

    let long_context_with_parent_first = [
        ContextFrame::new("other".to_string()),
        ContextFrame::new("another".to_string()),
        ContextFrame::new("parent".to_string()),
    ];

    assert!(
        matcher.matches("child", &long_context_with_parent_first),
        "Should match when parent is direct parent"
    );
}
