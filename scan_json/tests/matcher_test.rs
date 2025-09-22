use scan_json::matcher::{iter_match, debug_print_no_match};

// Helper function to create context iterators from string slices
fn context_from_strs<'a>(strs: &'a [&'a str]) -> impl Iterator<Item = &'a [u8]> + 'a {
    strs.iter().map(|s| s.as_bytes())
}

#[test]
fn test_iter_match_empty_iterator() {
    // Empty iterator always returns true
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), b"any", std::iter::empty()));
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), b"name", context_from_strs(&["parent"])));
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), b"field", context_from_strs(&["parent", "grandparent"])));
}

#[test]
fn test_iter_match_single_name() {
    // Should match when name matches and no context
    assert!(iter_match(|| [b"field"], b"field", std::iter::empty()));

    // Should not match when name doesn't match
    assert!(!iter_match(|| [b"field"], b"other", std::iter::empty()));

    // Should match with extra context
    assert!(iter_match(|| [b"field"], b"field", context_from_strs(&["parent"])));
}

#[test]
fn test_iter_match_name_and_parent() {
    // Should match when name and parent match
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", context_from_strs(&["parent"])));

    // Should not match when name is wrong
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"wrong", context_from_strs(&["parent"])));

    // Should not match when parent is wrong
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", context_from_strs(&["wrong"])));

    // Should not match when no context
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", std::iter::empty()));

    // Should match with extra context
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", context_from_strs(&["parent", "grandparent"])));
}

#[test]
fn test_iter_match_name_parent_grandparent() {
    // Should match when all levels match
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", context_from_strs(&["parent", "grandparent"])));

    // Should not match when name is wrong
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"wrong", context_from_strs(&["parent", "grandparent"])));

    // Should not match when parent is wrong
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", context_from_strs(&["wrong", "grandparent"])));

    // Should not match when grandparent is wrong
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", context_from_strs(&["parent", "wrong"])));

    // Should not match when insufficient context
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", context_from_strs(&["parent"])));

    // Should match with extra context
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", context_from_strs(&["parent", "grandparent", "great"])));
}

#[test]
fn test_iter_match_with_strings() {
    // Should work with string literals (converted to bytes)
    assert!(iter_match(|| ["field", "parent"], b"field", context_from_strs(&["parent"])));
    assert!(!iter_match(|| ["field", "parent"], b"field", context_from_strs(&["wrong"])));
}


#[test]
fn test_iter_match_reusable() {
    // Should be able to call the function multiple times
    assert!(iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"field", context_from_strs(&["parent"])));
    assert!(iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"field", context_from_strs(&["parent"])));
    assert!(!iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"wrong", context_from_strs(&["parent"])));
    assert!(!iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"wrong", context_from_strs(&["parent"])));
}

#[test]
fn test_iter_match_deep_nesting() {
    assert!(iter_match(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ], b"field", context_from_strs(&["level1", "level2", "level3", "level4"])));

    // Wrong at any level should fail
    assert!(!iter_match(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ], b"field", context_from_strs(&["level1", "wrong", "level3", "level4"])));
}

#[test]
fn test_debug_print_no_match_always_false() {
    // debug_print_no_match should always return false
    assert!(!debug_print_no_match(b"any", std::iter::empty()));
    assert!(!debug_print_no_match(b"name", context_from_strs(&["parent"])));
    assert!(!debug_print_no_match(b"field", context_from_strs(&["parent", "grandparent"])));
}