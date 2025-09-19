use scan_json::matcher::{IterMatcher, Matcher, DebugPrinter};

// Helper function to create context iterators from string slices
fn context_from_strs<'a>(strs: &'a [&'a str]) -> impl Iterator<Item = &'a [u8]> + 'a {
    strs.iter().map(|s| s.as_bytes())
}

#[test]
fn test_iter_matcher_empty_iterator() {
    let matcher = IterMatcher::new(|| std::iter::empty::<&[u8]>());

    // Empty iterator always returns true
    assert!(matcher.matches(b"any", std::iter::empty()));
    assert!(matcher.matches(b"name", context_from_strs(&["parent"])));
    assert!(matcher.matches(b"field", context_from_strs(&["parent", "grandparent"])));
}

#[test]
fn test_iter_matcher_single_name() {
    let matcher = IterMatcher::new(|| [b"field"]);

    // Should match when name matches and no context
    assert!(matcher.matches(b"field", std::iter::empty()));

    // Should not match when name doesn't match
    assert!(!matcher.matches(b"other", std::iter::empty()));

    // Should match with extra context
    assert!(matcher.matches(b"field", context_from_strs(&["parent"])));
}

#[test]
fn test_iter_matcher_name_and_parent() {
    let matcher = IterMatcher::new(|| ["child".as_bytes(), "parent".as_bytes()]);

    // Should match when name and parent match
    assert!(matcher.matches(b"child", context_from_strs(&["parent"])));

    // Should not match when name is wrong
    assert!(!matcher.matches(b"wrong", context_from_strs(&["parent"])));

    // Should not match when parent is wrong
    assert!(!matcher.matches(b"child", context_from_strs(&["wrong"])));

    // Should not match when no context
    assert!(!matcher.matches(b"child", std::iter::empty()));

    // Should match with extra context
    assert!(matcher.matches(b"child", context_from_strs(&["parent", "grandparent"])));
}

#[test]
fn test_iter_matcher_name_parent_grandparent() {
    let matcher = IterMatcher::new(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()]);

    // Should match when all levels match
    assert!(matcher.matches(b"child", context_from_strs(&["parent", "grandparent"])));

    // Should not match when name is wrong
    assert!(!matcher.matches(b"wrong", context_from_strs(&["parent", "grandparent"])));

    // Should not match when parent is wrong
    assert!(!matcher.matches(b"child", context_from_strs(&["wrong", "grandparent"])));

    // Should not match when grandparent is wrong
    assert!(!matcher.matches(b"child", context_from_strs(&["parent", "wrong"])));

    // Should not match when insufficient context
    assert!(!matcher.matches(b"child", context_from_strs(&["parent"])));

    // Should match with extra context
    assert!(matcher.matches(b"child", context_from_strs(&["parent", "grandparent", "great"])));
}

#[test]
fn test_iter_matcher_with_strings() {
    let matcher = IterMatcher::new(|| ["field", "parent"]);

    // Should work with string literals (converted to bytes)
    assert!(matcher.matches(b"field", context_from_strs(&["parent"])));
    assert!(!matcher.matches(b"field", context_from_strs(&["wrong"])));
}


#[test]
fn test_iter_matcher_reusable() {
    let matcher = IterMatcher::new(|| ["field".as_bytes(), "parent".as_bytes()]);

    // Should be able to call matches multiple times
    assert!(matcher.matches(b"field", context_from_strs(&["parent"])));
    assert!(matcher.matches(b"field", context_from_strs(&["parent"])));
    assert!(!matcher.matches(b"wrong", context_from_strs(&["parent"])));
    assert!(!matcher.matches(b"wrong", context_from_strs(&["parent"])));
}

#[test]
fn test_iter_matcher_deep_nesting() {
    let matcher = IterMatcher::new(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ]);

    assert!(matcher.matches(b"field", context_from_strs(&["level1", "level2", "level3", "level4"])));

    // Wrong at any level should fail
    assert!(!matcher.matches(b"field", context_from_strs(&["level1", "wrong", "level3", "level4"])));
}

#[test]
fn test_debug_printer_always_false() {
    let matcher = DebugPrinter;

    // DebugPrinter should always return false
    assert!(!matcher.matches(b"any", std::iter::empty()));
    assert!(!matcher.matches(b"name", context_from_strs(&["parent"])));
    assert!(!matcher.matches(b"field", context_from_strs(&["parent", "grandparent"])));
}