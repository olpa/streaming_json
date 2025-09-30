use scan_json::matcher::{iter_match, debug_print_no_match, StructuralPseudoname};
use scan_json::stack::ContextIter;
use scan_json::scan::StructurePosition;
use u8pool::U8Pool;

const S: StructurePosition = StructurePosition::ObjectMiddle;

#[test]
fn test_iter_match_empty_iterator() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Empty iterator always returns true - test with empty path
    pool.clear();
    pool.push_assoc(S, b"any").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), StructuralPseudoname::None, path));

    // Test with parent in path
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"name").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), StructuralPseudoname::None, path));

    // Test with grandparent in path
    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), StructuralPseudoname::None, path));
}

#[test]
fn test_iter_match_single_name() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should match when name matches and no parent in path
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| [b"field"], StructuralPseudoname::None, path));

    // Should not match when name doesn't match
    pool.clear();
    pool.push_assoc(S, b"other").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| [b"field"], StructuralPseudoname::None, path));

    // Should match with extra parent in path
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| [b"field"], StructuralPseudoname::None, path));
}

#[test]
fn test_iter_match_name_and_parent() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should match when name and parent match
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);

    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when name is wrong
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when parent is wrong
    pool.clear();
    pool.push_assoc(S, b"wrong").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when no parent in path
    pool.clear();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    // Should match with extra ancestor in path
    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);

    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));
}

#[test]
fn test_iter_match_name_parent_grandparent() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should match when all levels match
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);

    // Debug
    let debug_path = ContextIter::new(&pool);
    println!("Grandparent test path:");
    for (i, item) in debug_path.enumerate() {
        println!("  path[{}]: {:?}", i, std::str::from_utf8(item).unwrap_or("invalid utf8"));
    }

    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when name is wrong
    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when parent is wrong
    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"wrong").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when grandparent is wrong
    pool.clear();
    pool.push_assoc(S, b"wrong").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::None, path));

    // Should not match when insufficient ancestors in path
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::None, path));

    // Should match with extra ancestors in path
    pool.clear();
    pool.push_assoc(S, b"great").unwrap();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"child").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["child".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::None, path));
}

#[test]
fn test_iter_match_reusable() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Should be able to call the function multiple times
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["field".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["field".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    // Test with wrong name
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["field".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["field".as_bytes(), "parent".as_bytes()], StructuralPseudoname::None, path));
}

#[test]
fn test_iter_match_deep_nesting() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push_assoc(S, b"level4").unwrap();
    pool.push_assoc(S, b"level3").unwrap();
    pool.push_assoc(S, b"level2").unwrap();
    pool.push_assoc(S, b"level1").unwrap();
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ], StructuralPseudoname::None, path));

    // Wrong at any level should fail
    pool.clear();
    pool.push_assoc(S, b"level4").unwrap();
    pool.push_assoc(S, b"level3").unwrap();
    pool.push_assoc(S, b"wrong").unwrap();
    pool.push_assoc(S, b"level1").unwrap();
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| [
        "field".as_bytes(),
        "level1".as_bytes(),
        "level2".as_bytes(),
        "level3".as_bytes(),
        "level4".as_bytes()
    ], StructuralPseudoname::None, path));
}

#[test]
fn test_debug_print_no_match_always_false() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // debug_print_no_match should always return false
    let path = ContextIter::new(&pool);
    assert!(!debug_print_no_match(StructuralPseudoname::None, path));

    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"name").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!debug_print_no_match(StructuralPseudoname::None, path));

    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    pool.push_assoc(S, b"field").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!debug_print_no_match(StructuralPseudoname::None, path));
}

#[test]
fn test_iter_match_empty_iterator_structural_pseudonames() {
    let mut buffer = [0u8; 1024];
    let pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Empty iterator always returns true for all structural pseudonames
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), StructuralPseudoname::Array, path));

    let path = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), StructuralPseudoname::Object, path));

    let path = ContextIter::new(&pool);
    assert!(iter_match(|| std::iter::empty::<&[u8]>(), StructuralPseudoname::Atom, path));
}

#[test]
fn test_iter_match_structural_pseudonames() {
    let mut buffer = [0u8; 1024];
    let pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test Array pseudoname
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#array".as_bytes()], StructuralPseudoname::Array, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#object".as_bytes()], StructuralPseudoname::Array, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#atom".as_bytes()], StructuralPseudoname::Array, path));

    // Test Object pseudoname
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#object".as_bytes()], StructuralPseudoname::Object, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#array".as_bytes()], StructuralPseudoname::Object, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#atom".as_bytes()], StructuralPseudoname::Object, path));

    // Test Atom pseudoname
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#atom".as_bytes()], StructuralPseudoname::Atom, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#array".as_bytes()], StructuralPseudoname::Atom, path));

    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#object".as_bytes()], StructuralPseudoname::Atom, path));
}

#[test]
fn test_iter_match_structural_pseudonames_with_context() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test Array with parent context
    pool.push_assoc(S, b"parent").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#array".as_bytes(), "parent".as_bytes()], StructuralPseudoname::Array, path));

    pool.clear();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#array".as_bytes(), "parent".as_bytes()], StructuralPseudoname::Array, path));

    // Test Object with parent context
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#object".as_bytes(), "parent".as_bytes()], StructuralPseudoname::Object, path));

    pool.clear();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#object".as_bytes(), "parent".as_bytes()], StructuralPseudoname::Object, path));

    // Test Atom with parent context
    pool.clear();
    pool.push_assoc(S, b"parent").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#atom".as_bytes(), "parent".as_bytes()], StructuralPseudoname::Atom, path));

    pool.clear();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#atom".as_bytes(), "parent".as_bytes()], StructuralPseudoname::Atom, path));

    // Test with multiple levels
    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"parent").unwrap();
    let path = ContextIter::new(&pool);
    assert!(iter_match(|| ["#array".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::Array, path));

    pool.clear();
    pool.push_assoc(S, b"grandparent").unwrap();
    pool.push_assoc(S, b"wrong").unwrap();
    let path = ContextIter::new(&pool);
    assert!(!iter_match(|| ["#array".as_bytes(), "parent".as_bytes(), "grandparent".as_bytes()], StructuralPseudoname::Array, path));
}
