use scan_json::matcher::{iter_match, debug_print_no_match};
use scan_json::stack::ContextIter;
use u8pool::U8Pool;

#[test]
fn test_iter_match_empty_iterator() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Empty iterator always returns true - test with empty context
    pool.clear();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), b"any", ctx));

    // Test with parent context
    pool.clear();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), b"name", ctx));

    // Test with grandparent context
    pool.clear();
    pool.push(b"grandparent").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), b"field", ctx));
}

#[test]
fn test_iter_match_single_name() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should match when name matches and no context
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| [b"field"], b"field", ctx));

    // Should not match when name doesn't match
    pool.clear();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| [b"field"], b"other", ctx));

    // Should match with extra context
    pool.clear();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| [b"field"], b"field", ctx));
}

#[test]
fn test_iter_match_name_and_parent() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should match when name and parent match
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", ctx));

    // Should not match when name is wrong
    pool.clear();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"wrong", ctx));

    // Should not match when parent is wrong
    pool.clear();
    pool.push(b"wrong").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", ctx));

    // Should not match when no context
    pool.clear();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", ctx));

    // Should match with extra context
    pool.clear();
    pool.push(b"grandparent").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes()], b"child", ctx));
}

#[test]
fn test_iter_match_name_parent_grandparent() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should match when all levels match
    pool.push(b"grandparent").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", ctx));

    // Should not match when name is wrong
    pool.clear();
    pool.push(b"grandparent").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"wrong", ctx));

    // Should not match when parent is wrong
    pool.clear();
    pool.push(b"grandparent").unwrap();
    pool.push(b"wrong").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", ctx));

    // Should not match when grandparent is wrong
    pool.clear();
    pool.push(b"wrong").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", ctx));

    // Should not match when insufficient context
    pool.clear();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", ctx));

    // Should match with extra context
    pool.clear();
    pool.push(b"great").unwrap();
    pool.push(b"grandparent").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], b"child", ctx));
}

#[test]
fn test_iter_match_with_strings() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should work with string literals (converted to bytes)
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["field", "parent"], b"field", ctx));

    pool.clear();
    pool.push(b"wrong").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["field", "parent"], b"field", ctx));
}

#[test]
fn test_iter_match_reusable() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should be able to call the function multiple times
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"field", ctx));

    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"field", ctx));

    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"wrong", ctx));

    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| ["field".as_bytes(), "parent".as_bytes()], b"wrong", ctx));
}

#[test]
fn test_iter_match_deep_nesting() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push(b"level4").unwrap();
    pool.push(b"level3").unwrap();
    pool.push(b"level2").unwrap();
    pool.push(b"level1").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(iter_match(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ], b"field", ctx));

    // Wrong at any level should fail
    pool.clear();
    pool.push(b"level4").unwrap();
    pool.push(b"level3").unwrap();
    pool.push(b"wrong").unwrap();
    pool.push(b"level1").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!iter_match(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ], b"field", ctx));
}

#[test]
fn test_debug_print_no_match_always_false() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // debug_print_no_match should always return false
    let ctx = ContextIter::new(&pool);
    assert!(!debug_print_no_match(b"any", ctx));

    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!debug_print_no_match(b"name", ctx));

    pool.clear();
    pool.push(b"grandparent").unwrap();
    pool.push(b"parent").unwrap();
    let ctx = ContextIter::new(&pool);
    assert!(!debug_print_no_match(b"field", ctx));
}
