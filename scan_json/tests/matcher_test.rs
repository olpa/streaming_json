use scan_json::matcher::{Matcher, Name, ParentAndName};
use scan_json::scan::mk_context_frame_for_test;

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
    let matcher = ParentAndName::new("parent".to_string(), "child".to_string());

    let empty_context = [];
    assert!(
        !matcher.matches("child", &empty_context),
        "Should not match without context"
    );

    let wrong_parent_context = [mk_context_frame_for_test("wrong".to_string())];
    assert!(
        !matcher.matches("child", &wrong_parent_context),
        "Should not match with wrong parent"
    );

    let wrong_name_context = [mk_context_frame_for_test("parent".to_string())];
    assert!(
        !matcher.matches("wrong", &wrong_name_context),
        "Should not match with wrong name"
    );

    let matching_context = [mk_context_frame_for_test("parent".to_string())];
    assert!(
        matcher.matches("child", &matching_context),
        "Should match with correct parent and name"
    );
}

#[test]
fn test_match_by_parent_and_name_long_context() {
    use scan_json::matcher::ParentAndName;

    let matcher = ParentAndName::new("parent".to_string(), "child".to_string());

    let long_context = [
        mk_context_frame_for_test("grandparent".to_string()),
        mk_context_frame_for_test("parent".to_string()),
        mk_context_frame_for_test("child".to_string()),
    ];

    assert!(
        !matcher.matches("child", &long_context),
        "Should not match when parent is deeper in context"
    );

    let long_context_with_parent_first = [
        mk_context_frame_for_test("other".to_string()),
        mk_context_frame_for_test("another".to_string()),
        mk_context_frame_for_test("parent".to_string()),
    ];

    assert!(
        matcher.matches("child", &long_context_with_parent_first),
        "Should match when parent is direct parent"
    );
}
