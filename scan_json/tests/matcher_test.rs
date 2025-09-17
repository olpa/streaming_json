use scan_json::matcher::{IterMatcher, Matcher, DebugPrinter};

#[test]
fn test_iter_matcher_empty_iterator() {
    let matcher = IterMatcher::new(|| std::iter::empty::<&[u8]>());

    // Empty iterator always returns true
    assert!(matcher.matches(b"any", std::iter::empty()));
    assert!(matcher.matches(b"name", [b"parent"].iter().copied()));
    assert!(matcher.matches(b"field", [b"parent", b"grandparent"].iter().copied()));
}

#[test]
fn test_iter_matcher_single_name() {
    let matcher = IterMatcher::new(|| [b"field"]);

    // Should match when name matches and no context
    assert!(matcher.matches(b"field", std::iter::empty()));

    // Should not match when name doesn't match
    assert!(!matcher.matches(b"other", std::iter::empty()));

    // Should not match when there's extra context
    assert!(!matcher.matches(b"field", [b"parent"].iter().copied()));
}

#[test]
fn test_iter_matcher_name_and_parent() {
    let matcher = IterMatcher::new(|| [b"child", b"parent"]);

    // Should match when name and parent match
    assert!(matcher.matches(b"child", [b"parent"].iter().copied()));

    // Should not match when name is wrong
    assert!(!matcher.matches(b"wrong", [b"parent"].iter().copied()));

    // Should not match when parent is wrong
    assert!(!matcher.matches(b"child", [b"wrong"].iter().copied()));

    // Should not match when no context
    assert!(!matcher.matches(b"child", std::iter::empty()));

    // Should not match when extra context
    assert!(!matcher.matches(b"child", [b"parent", b"grandparent"].iter().copied()));
}

#[test]
fn test_iter_matcher_name_parent_grandparent() {
    let matcher = IterMatcher::new(|| [b"child", b"parent", b"grandparent"]);

    // Should match when all levels match
    assert!(matcher.matches(b"child", [b"parent", b"grandparent"].iter().copied()));

    // Should not match when name is wrong
    assert!(!matcher.matches(b"wrong", [b"parent", b"grandparent"].iter().copied()));

    // Should not match when parent is wrong
    assert!(!matcher.matches(b"child", [b"wrong", b"grandparent"].iter().copied()));

    // Should not match when grandparent is wrong
    assert!(!matcher.matches(b"child", [b"parent", b"wrong"].iter().copied()));

    // Should not match when insufficient context
    assert!(!matcher.matches(b"child", [b"parent"].iter().copied()));

    // Should not match when extra context
    assert!(!matcher.matches(b"child", [b"parent", b"grandparent", b"great"].iter().copied()));
}

#[test]
fn test_iter_matcher_with_strings() {
    let matcher = IterMatcher::new(|| ["field", "parent"]);

    // Should work with string literals (converted to bytes)
    assert!(matcher.matches(b"field", [b"parent"].iter().copied()));
    assert!(!matcher.matches(b"field", [b"wrong"].iter().copied()));
}

#[test]
fn test_iter_matcher_with_mixed_types() {
    let matcher = IterMatcher::new(|| {
        vec![
            "field".to_string(),
            String::from("parent")
        ]
    });

    // Should work with owned strings
    assert!(matcher.matches(b"field", [b"parent"].iter().copied()));
    assert!(!matcher.matches(b"wrong", [b"parent"].iter().copied()));
}

#[test]
fn test_iter_matcher_reusable() {
    let matcher = IterMatcher::new(|| [b"field", b"parent"]);

    // Should be able to call matches multiple times
    assert!(matcher.matches(b"field", [b"parent"].iter().copied()));
    assert!(matcher.matches(b"field", [b"parent"].iter().copied()));
    assert!(!matcher.matches(b"wrong", [b"parent"].iter().copied()));
    assert!(!matcher.matches(b"wrong", [b"parent"].iter().copied()));
}

#[test]
fn test_iter_matcher_deep_nesting() {
    let matcher = IterMatcher::new(|| [
        b"field",
        b"level1",
        b"level2",
        b"level3",
        b"level4"
    ]);

    let context = [b"level1", b"level2", b"level3", b"level4"];
    assert!(matcher.matches(b"field", context.iter().copied()));

    // Wrong at any level should fail
    let wrong_context = [b"level1", b"wrong", b"level3", b"level4"];
    assert!(!matcher.matches(b"field", wrong_context.iter().copied()));
}

#[test]
fn test_debug_printer_always_false() {
    let matcher = DebugPrinter;

    // DebugPrinter should always return false
    assert!(!matcher.matches(b"any", std::iter::empty()));
    assert!(!matcher.matches(b"name", [b"parent"].iter().copied()));
    assert!(!matcher.matches(b"field", [b"parent", b"grandparent"].iter().copied()));
}