/// Tests for deeply nested JSON structures
/// DynamoDB supports up to 32 levels of nesting

// Helper function to generate nested object JSON with specified depth
// Creates structure like: {"level_0": {"level_1": {"level_2": ... {"value": "leaf"}}}}
fn generate_nested_object_normal_json(depth: usize) -> String {
    if depth == 0 {
        return r#"{"value":"leaf"}"#.to_string();
    }

    let mut json = String::from("{");
    for i in 0..depth {
        json.push_str(&format!(r#""level_{}":{{"#, i));
    }
    json.push_str(r#""value":"leaf""#);
    for _ in 0..depth {
        json.push_str("}");
    }
    json.push_str("}");
    json
}

#[test]
fn test_helper_generate_nested_object_depth_0() {
    let result = generate_nested_object_normal_json(0);
    assert_eq!(result, r#"{"value":"leaf"}"#);
}

#[test]
fn test_helper_generate_nested_object_depth_1() {
    let result = generate_nested_object_normal_json(1);
    assert_eq!(result, r#"{"level_0":{"value":"leaf"}}"#);
}

#[test]
fn test_helper_generate_nested_object_depth_2() {
    let result = generate_nested_object_normal_json(2);
    assert_eq!(result, r#"{"level_0":{"level_1":{"value":"leaf"}}}"#);
}

#[test]
fn test_helper_generate_nested_object_ddb_depth_0() {
    let result = generate_nested_object_ddb_json(0, true);
    assert_eq!(result, "{\"Item\":{\"value\":{\"S\":\"leaf\"}}}\n");
}

#[test]
fn test_helper_generate_nested_object_ddb_depth_1() {
    let result = generate_nested_object_ddb_json(1, true);
    assert_eq!(
        result,
        "{\"Item\":{\"level_0\":{\"M\":{\"value\":{\"S\":\"leaf\"}}}}}\n"
    );
}

#[test]
fn test_helper_generate_nested_object_ddb_depth_2() {
    let result = generate_nested_object_ddb_json(2, true);
    assert_eq!(
        result,
        "{\"Item\":{\"level_0\":{\"M\":{\"level_1\":{\"M\":{\"value\":{\"S\":\"leaf\"}}}}}}}\n"
    );
}

// Helper function to generate expected DynamoDB JSON for nested objects
fn generate_nested_object_ddb_json(depth: usize, with_item_wrapper: bool) -> String {
    let mut json = String::new();
    if with_item_wrapper {
        json.push_str(r#"{"Item":"#);
    }

    json.push_str("{");
    for i in 0..depth {
        json.push_str(&format!(r#""level_{}":{{"M":{{"#, i));
    }
    json.push_str(r#""value":{"S":"leaf"}"#);
    for _ in 0..depth {
        json.push_str("}}");
    }
    json.push_str("}");

    if with_item_wrapper {
        json.push_str("}");
    }
    json.push_str("\n");
    json
}

// Helper function to generate nested array JSON with specified depth
// Creates structure like: [[[["value"]]]]
// depth=1: {"data":["value"]}, depth=2: {"data":[["value"]]}
fn generate_nested_array_normal_json(depth: usize) -> String {
    let mut json = String::from(r#"{"data":"#);
    for _ in 0..depth {
        json.push('[');
    }
    json.push_str(r#""value""#);
    for _ in 0..depth {
        json.push(']');
    }
    json.push_str("}");
    json
}

// Helper function to generate expected DynamoDB JSON for nested arrays
// depth=1: {"data":{"L":[{"S":"value"}]}}, depth=2: {"data":{"L":[{"L":[{"S":"value"}]}]}}
fn generate_nested_array_ddb_json(depth: usize, with_item_wrapper: bool) -> String {
    let mut json = String::new();
    if with_item_wrapper {
        json.push_str(r#"{"Item":"#);
    }

    json.push_str(r#"{"data":{"L":"#);
    // First level is the outer array
    json.push('[');
    // For depth > 1, we need nested L types
    for _ in 1..depth {
        json.push_str(r#"{"L":"#);
        json.push('[');
    }
    // The innermost value
    json.push_str(r#"{"S":"value"}"#);
    // Close nested arrays
    for _ in 1..depth {
        json.push_str("]}");
    }
    json.push_str("]}}");

    if with_item_wrapper {
        json.push_str("}");
    }
    json.push_str("\n");
    json
}

// Helper function to convert normal JSON to DDB JSON for testing
fn convert_to_ddb_test(normal_json: &str, with_item_wrapper: bool) -> String {
    let mut reader = normal_json.as_bytes();
    let mut output = vec![0u8; 16384]; // Larger buffer for deeply nested structures
    let mut output_slice = output.as_mut_slice();
    let mut rjiter_buffer = [0u8; 4096];
    let mut context_buffer = [0u8; 2048];

    ddb_convert::convert_normal_to_ddb(
        &mut reader,
        &mut output_slice,
        &mut rjiter_buffer,
        &mut context_buffer,
        false,
        false,
        with_item_wrapper,
    )
    .unwrap();

    let bytes_written = 16384 - output_slice.len();
    std::str::from_utf8(&output[..bytes_written])
        .unwrap()
        .to_string()
}

// Helper function to convert DDB JSON to normal JSON for testing
fn convert_from_ddb_test(ddb_json: &str) -> String {
    let mut reader = ddb_json.as_bytes();
    let mut output = vec![0u8; 16384]; // Larger buffer for deeply nested structures
    let mut output_slice = output.as_mut_slice();
    let mut rjiter_buffer = [0u8; 4096];
    let mut context_buffer = [0u8; 2048];

    ddb_convert::convert_ddb_to_normal(
        &mut reader,
        &mut output_slice,
        &mut rjiter_buffer,
        &mut context_buffer,
        false,
        false,
        ddb_convert::ItemWrapperMode::AsWrapper,
    )
    .unwrap();

    let bytes_written = 16384 - output_slice.len();
    std::str::from_utf8(&output[..bytes_written])
        .unwrap()
        .to_string()
}

// ===== Tests for Normal JSON to DynamoDB JSON =====

#[test]
fn test_to_ddb_nested_objects_depth_1() {
    let input = generate_nested_object_normal_json(1);
    let expected = generate_nested_object_ddb_json(1, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_objects_depth_16() {
    let input = generate_nested_object_normal_json(16);
    let expected = generate_nested_object_ddb_json(16, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_objects_depth_31() {
    let input = generate_nested_object_normal_json(31);
    let expected = generate_nested_object_ddb_json(31, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_objects_depth_32() {
    // DynamoDB maximum nesting level is 32
    let input = generate_nested_object_normal_json(32);
    let expected = generate_nested_object_ddb_json(32, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_arrays_depth_1() {
    let input = generate_nested_array_normal_json(1);
    let expected = generate_nested_array_ddb_json(1, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_arrays_depth_16() {
    let input = generate_nested_array_normal_json(16);
    let expected = generate_nested_array_ddb_json(16, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_arrays_depth_31() {
    let input = generate_nested_array_normal_json(31);
    let expected = generate_nested_array_ddb_json(31, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_arrays_depth_32() {
    // DynamoDB maximum nesting level is 32
    let input = generate_nested_array_normal_json(32);
    let expected = generate_nested_array_ddb_json(32, true);
    let result = convert_to_ddb_test(&input, true);
    assert_eq!(result, expected);
}

// ===== Tests for DynamoDB JSON to Normal JSON =====

#[test]
fn test_from_ddb_nested_objects_depth_1() {
    let input = generate_nested_object_ddb_json(1, true);
    let expected = generate_nested_object_normal_json(1) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_objects_depth_16() {
    let input = generate_nested_object_ddb_json(16, true);
    let expected = generate_nested_object_normal_json(16) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_objects_depth_31() {
    let input = generate_nested_object_ddb_json(31, true);
    let expected = generate_nested_object_normal_json(31) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_objects_depth_32() {
    // DynamoDB maximum nesting level is 32
    let input = generate_nested_object_ddb_json(32, true);
    let expected = generate_nested_object_normal_json(32) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_arrays_depth_1() {
    let input = generate_nested_array_ddb_json(1, true);
    let expected = generate_nested_array_normal_json(1) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_arrays_depth_16() {
    let input = generate_nested_array_ddb_json(16, true);
    let expected = generate_nested_array_normal_json(16) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_arrays_depth_31() {
    let input = generate_nested_array_ddb_json(31, true);
    let expected = generate_nested_array_normal_json(31) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_nested_arrays_depth_32() {
    // DynamoDB maximum nesting level is 32
    let input = generate_nested_array_ddb_json(32, true);
    let expected = generate_nested_array_normal_json(32) + "\n";
    let result = convert_from_ddb_test(&input);
    assert_eq!(result, expected);
}

// ===== Roundtrip Tests =====

#[test]
fn test_roundtrip_nested_objects_depth_16() {
    let original = generate_nested_object_normal_json(16);

    // Convert to DDB
    let ddb_result = convert_to_ddb_test(&original, true);

    // Convert back to normal
    let roundtrip_result = convert_from_ddb_test(&ddb_result);

    // Should match original (plus newline)
    assert_eq!(roundtrip_result, original + "\n");
}

#[test]
fn test_roundtrip_nested_objects_depth_32() {
    let original = generate_nested_object_normal_json(32);

    // Convert to DDB
    let ddb_result = convert_to_ddb_test(&original, true);

    // Convert back to normal
    let roundtrip_result = convert_from_ddb_test(&ddb_result);

    // Should match original (plus newline)
    assert_eq!(roundtrip_result, original + "\n");
}

#[test]
fn test_roundtrip_nested_arrays_depth_16() {
    let original = generate_nested_array_normal_json(16);

    // Convert to DDB
    let ddb_result = convert_to_ddb_test(&original, true);

    // Convert back to normal
    let roundtrip_result = convert_from_ddb_test(&ddb_result);

    // Should match original (plus newline)
    assert_eq!(roundtrip_result, original + "\n");
}

#[test]
fn test_roundtrip_nested_arrays_depth_32() {
    let original = generate_nested_array_normal_json(32);

    // Convert to DDB
    let ddb_result = convert_to_ddb_test(&original, true);

    // Convert back to normal
    let roundtrip_result = convert_from_ddb_test(&ddb_result);

    // Should match original (plus newline)
    assert_eq!(roundtrip_result, original + "\n");
}

// ===== Mixed Nesting Tests =====

#[test]
fn test_to_ddb_mixed_nesting_objects_and_arrays() {
    // Mix of objects and arrays: {"a": {"b": [{"c": ["value"]}]}}
    let input = r#"{"a":{"b":[{"c":["value"]}]}}"#;
    let expected = r#"{"Item":{"a":{"M":{"b":{"L":[{"M":{"c":{"L":[{"S":"value"}]}}}]}}}}}
"#;
    let result = convert_to_ddb_test(input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_mixed_nesting_objects_and_arrays() {
    let input = r#"{"Item":{"a":{"M":{"b":{"L":[{"M":{"c":{"L":[{"S":"value"}]}}}]}}}}}"#;
    let expected = r#"{"a":{"b":[{"c":["value"]}]}}
"#;
    let result = convert_from_ddb_test(input);
    assert_eq!(result, expected);
}

#[test]
fn test_roundtrip_complex_mixed_structure() {
    // Complex structure with multiple levels and types
    let original = r#"{"users":[{"id":"user1","settings":{"preferences":{"theme":"dark","notifications":[{"type":"email","enabled":true}]}}}]}"#;

    // Convert to DDB
    let ddb_result = convert_to_ddb_test(original, true);

    // Convert back to normal
    let roundtrip_result = convert_from_ddb_test(&ddb_result);

    // Should match original (plus newline)
    assert_eq!(roundtrip_result, original.to_string() + "\n");
}

// ===== Edge Cases =====

#[test]
fn test_to_ddb_empty_nested_structures() {
    // Empty objects and arrays at various depths
    let input = r#"{"a":{"b":{"c":{}}}}"#;
    let expected = r#"{"Item":{"a":{"M":{"b":{"M":{"c":{"M":{}}}}}}}}
"#;
    let result = convert_to_ddb_test(input, true);
    assert_eq!(result, expected);
}

#[test]
fn test_from_ddb_empty_nested_structures() {
    let input = r#"{"Item":{"a":{"M":{"b":{"M":{"c":{"M":{}}}}}}}}"#;
    let expected = r#"{"a":{"b":{"c":{}}}}
"#;
    let result = convert_from_ddb_test(input);
    assert_eq!(result, expected);
}

#[test]
fn test_nested_objects_without_item_wrapper() {
    let input = generate_nested_object_normal_json(5);
    let expected = generate_nested_object_ddb_json(5, false);
    let result = convert_to_ddb_test(&input, false);
    assert_eq!(result, expected);
}
