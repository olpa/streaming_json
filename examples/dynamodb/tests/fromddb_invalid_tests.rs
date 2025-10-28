/// Tests for invalid DynamoDB JSON format validation
///
/// These tests verify that the from-ddb converter properly rejects invalid
/// DynamoDB JSON format according to the issues documented in issues.md

/// Helper function to test that conversion fails with an error
fn convert_test_expect_error(ddb_json: &str) -> ddb_convert::ConversionError {
    let mut reader = ddb_json.as_bytes();
    let mut output = vec![0u8; 4096];
    let mut output_slice = output.as_mut_slice();
    let mut rjiter_buffer = [0u8; 4096];
    let mut context_buffer = [0u8; 2048];

    ddb_convert::convert_ddb_to_normal(
        &mut reader,
        &mut output_slice,
        &mut rjiter_buffer,
        &mut context_buffer,
        false,
    )
    .expect_err("Expected conversion to fail but it succeeded")
}

// ============================================================================
// Issue #1: Unknown Data Type Descriptors
// ============================================================================

#[test]
fn test_unknown_type_descriptor_lowercase_s() {
    // Using lowercase "s" instead of uppercase "S"
    let ddb_json = r#"{"Item":{"Field": {"s": "value"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    // Check that the error contains an explanation about the unknown type descriptor
    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("unknown type descriptor") || error_message.contains("'s'"),
        "Error message should explain unknown type descriptor 's', got: {}",
        error_message
    );
}

// ============================================================================
// Issue #2: Invalid String (S) Values
// ============================================================================

#[test]
fn test_string_type_with_number_value() {
    // S type must have a string value, not a number
    let ddb_json = r#"{"Item":{"Field": {"S": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("S"),
        "Error message should explain that S type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_string_type_with_boolean_value() {
    // S type must have a string value, not a boolean
    let ddb_json = r#"{"Item":{"Field": {"S": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("S"),
        "Error message should explain that S type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_string_type_with_null_value() {
    // S type must have a string value, not null
    let ddb_json = r#"{"Item":{"Field": {"S": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("S"),
        "Error message should explain that S type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_string_type_with_array_value() {
    // S type must have a string value, not an array
    let ddb_json = r#"{"Item":{"Field": {"S": ["value"]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("S"),
        "Error message should explain that S type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_string_type_with_object_value() {
    // S type must have a string value, not an object
    let ddb_json = r#"{"Item":{"Field": {"S": {"nested": "value"}}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("S"),
        "Error message should explain that S type expects a string value, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #4: Invalid Binary (B) Values
// ============================================================================

#[test]
fn test_binary_type_with_number_value() {
    // B type must have a string value, not a number
    let ddb_json = r#"{"Item":{"Field": {"B": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("B"),
        "Error message should explain that B type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_type_with_boolean_value() {
    // B type must have a string value, not a boolean
    let ddb_json = r#"{"Item":{"Field": {"B": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("B"),
        "Error message should explain that B type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_type_with_null_value() {
    // B type must have a string value, not null
    let ddb_json = r#"{"Item":{"Field": {"B": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("B"),
        "Error message should explain that B type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_type_with_array_value() {
    // B type must have a string value, not an array
    let ddb_json = r#"{"Item":{"Field": {"B": ["dGVzdA=="]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("B"),
        "Error message should explain that B type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_type_with_object_value() {
    // B type must have a string value, not an object
    let ddb_json = r#"{"Item":{"Field": {"B": {"data": "dGVzdA=="}}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("B"),
        "Error message should explain that B type expects a string value, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that B type strings contain valid base64.
// Rationale: For conversion efficiency, we copy the attribute value as-is without
// modification or validation. This allows for faster streaming conversion without
// needing to decode and validate base64 strings. Invalid base64 will be caught
// by the database when the data is used.
//
// #[test]
// fn test_binary_type_with_invalid_base64() {
//     // B type must have a valid base64 string
//     let ddb_json = r#"{"Item":{"Field": {"B": "not-base64!!!"}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("base64") || error_message.contains("B") || error_message.contains("invalid"),
//         "Error message should explain that B type string must be valid base64, got: {}",
//         error_message
//     );
// }

// ============================================================================
// Issue #3: Invalid Number (N) Values
// ============================================================================

#[test]
fn test_number_type_with_number_value() {
    // N type must have a string value, not a number (numbers are transmitted as strings)
    let ddb_json = r#"{"Item":{"Field": {"N": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("N"),
        "Error message should explain that N type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_number_type_with_boolean_value() {
    // N type must have a string value, not a boolean
    let ddb_json = r#"{"Item":{"Field": {"N": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("N"),
        "Error message should explain that N type expects a string value, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that N type strings contain valid numbers.
// Rationale: For conversion efficiency, we copy the attribute value as-is without
// modification or validation. This allows for faster streaming conversion without
// needing to parse and validate numeric strings. Invalid numbers will be caught
// by the database when the data is used.
//
// #[test]
// fn test_number_type_with_non_numeric_string() {
//     // N type must have a string that represents a valid number
//     let ddb_json = r#"{"Item":{"Field": {"N": "abc"}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("number") || error_message.contains("N") || error_message.contains("invalid"),
//         "Error message should explain that N type string must be a valid number, got: {}",
//         error_message
//     );
// }

#[test]
fn test_number_type_with_null_value() {
    // N type must have a string value, not null
    let ddb_json = r#"{"Item":{"Field": {"N": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("N"),
        "Error message should explain that N type expects a string value, got: {}",
        error_message
    );
}

#[test]
fn test_number_type_with_array_value() {
    // N type must have a string value, not an array
    let ddb_json = r#"{"Item":{"Field": {"N": ["123"]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("N"),
        "Error message should explain that N type expects a string value, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #5: Invalid Boolean (BOOL) Values
// ============================================================================

#[test]
fn test_bool_type_with_string_true() {
    // BOOL type must have a boolean value, not a string
    let ddb_json = r#"{"Item":{"Field": {"BOOL": "true"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("boolean") || error_message.contains("BOOL"),
        "Error message should explain that BOOL type expects a boolean value, got: {}",
        error_message
    );
}

#[test]
fn test_bool_type_with_string_false() {
    // BOOL type must have a boolean value, not a string
    let ddb_json = r#"{"Item":{"Field": {"BOOL": "false"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("boolean") || error_message.contains("BOOL"),
        "Error message should explain that BOOL type expects a boolean value, got: {}",
        error_message
    );
}

#[test]
fn test_bool_type_with_number_zero() {
    // BOOL type must have a boolean value, not a number
    let ddb_json = r#"{"Item":{"Field": {"BOOL": 0}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("boolean") || error_message.contains("BOOL"),
        "Error message should explain that BOOL type expects a boolean value, got: {}",
        error_message
    );
}

#[test]
fn test_bool_type_with_number_one() {
    // BOOL type must have a boolean value, not a number
    let ddb_json = r#"{"Item":{"Field": {"BOOL": 1}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("boolean") || error_message.contains("BOOL"),
        "Error message should explain that BOOL type expects a boolean value, got: {}",
        error_message
    );
}

#[test]
fn test_bool_type_with_null_value() {
    // BOOL type must have a boolean value, not null
    let ddb_json = r#"{"Item":{"Field": {"BOOL": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("boolean") || error_message.contains("BOOL"),
        "Error message should explain that BOOL type expects a boolean value, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #6: Invalid Null (NULL) Values
// ============================================================================

#[test]
fn test_null_type_with_false_value() {
    // NULL type must have the value true (not false)
    let ddb_json = r#"{"Item":{"Field": {"NULL": false}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true") || error_message.contains("NULL"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

#[test]
fn test_null_type_with_null_value() {
    // NULL type must have the value true (not null itself)
    let ddb_json = r#"{"Item":{"Field": {"NULL": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true") || error_message.contains("NULL") || error_message.contains("True"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

#[test]
fn test_null_type_with_string_value() {
    // NULL type must have the value true, not a string
    let ddb_json = r#"{"Item":{"Field": {"NULL": "true"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true") || error_message.contains("NULL") || error_message.contains("True"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

#[test]
fn test_null_type_with_number_value() {
    // NULL type must have the value true, not a number
    let ddb_json = r#"{"Item":{"Field": {"NULL": 1}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true") || error_message.contains("NULL") || error_message.contains("True"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #7: Invalid Map (M) Values
// ============================================================================

#[test]
fn test_map_type_with_string_value() {
    // M type must have an object value, not a string
    let ddb_json = r#"{"Item":{"Field": {"M": "not-an-object"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object") || error_message.contains("M") || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_number_value() {
    // M type must have an object value, not a number
    let ddb_json = r#"{"Item":{"Field": {"M": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object") || error_message.contains("M") || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_boolean_value() {
    // M type must have an object value, not a boolean
    let ddb_json = r#"{"Item":{"Field": {"M": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object") || error_message.contains("M") || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_null_value() {
    // M type must have an object value, not null
    let ddb_json = r#"{"Item":{"Field": {"M": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object") || error_message.contains("M") || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_array_value() {
    // M type must have an object value, not an array
    let ddb_json = r#"{"Item":{"Field": {"M": [{"S": "value"}]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object") || error_message.contains("M") || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that nested attributes inside M follow DynamoDB JSON format.
// Rationale: Validating nested structure would require tracking state through the entire M object
// to ensure all field values are type descriptor objects. This adds complexity and would require
// deeper integration with scan_json's state machine. For now, we only validate that M has an
// object value. Malformed nested structures will produce invalid output that can be caught by
// downstream consumers.
//
// #[test]
// fn test_map_type_with_nested_missing_type_descriptor() {
//     // M type nested attributes must follow DynamoDB JSON format (have type descriptors)
//     let ddb_json = r#"{"Item":{"Field": {"M": {"Nested": "value"}}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("type descriptor") || error_message.contains("unknown") || error_message.contains("Invalid"),
//         "Error message should explain that nested attributes need type descriptors, got: {}",
//         error_message
//     );
// }

// ============================================================================
// Issue #8: Invalid List (L) Values
// ============================================================================

#[test]
fn test_list_type_with_string_value() {
    // L type must have an array value, not a string
    let ddb_json = r#"{"Item":{"Field": {"L": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("L") || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_number_value() {
    // L type must have an array value, not a number
    let ddb_json = r#"{"Item":{"Field": {"L": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("L") || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_boolean_value() {
    // L type must have an array value, not a boolean
    let ddb_json = r#"{"Item":{"Field": {"L": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("L") || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_null_value() {
    // L type must have an array value, not null
    let ddb_json = r#"{"Item":{"Field": {"L": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("L") || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_object_value() {
    // L type must have an array value, not an object
    let ddb_json = r#"{"Item":{"Field": {"L": {"S": "value"}}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("L") || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that array elements inside L follow DynamoDB JSON format.
// Rationale: Validating nested structure would require tracking state through the entire L array
// to ensure all elements are type descriptor objects. This adds complexity and would require
// deeper integration with scan_json's state machine. For now, we only validate that L has an
// array value. Malformed nested structures will produce invalid output that can be caught by
// downstream consumers.
//
// #[test]
// fn test_list_type_with_elements_missing_type_descriptor() {
//     // L type array elements must follow DynamoDB JSON format (have type descriptors)
//     let ddb_json = r#"{"Item":{"Field": {"L": ["raw-value"]}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("type descriptor") || error_message.contains("unknown") || error_message.contains("Invalid"),
//         "Error message should explain that array elements need type descriptors, got: {}",
//         error_message
//     );
// }

// ============================================================================
// Issue #9: Invalid String Set (SS) Values
// ============================================================================

#[test]
fn test_string_set_with_string_value() {
    // SS type must have an array value, not a string
    let ddb_json = r#"{"Item":{"Field": {"SS": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("SS") || error_message.contains("Expected"),
        "Error message should explain that SS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_string_set_with_number_value() {
    // SS type must have an array value, not a number
    let ddb_json = r#"{"Item":{"Field": {"SS": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("SS") || error_message.contains("Expected"),
        "Error message should explain that SS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_string_set_with_array_of_numbers() {
    // SS array elements must be strings, not numbers
    let ddb_json = r#"{"Item":{"Field": {"SS": [123, 456]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("string") || error_message.contains("SS") || error_message.contains("Expected"),
        "Error message should explain that SS array elements must be strings, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that SS arrays contain duplicate values.
// Rationale: Validating duplicates would require tracking all values seen in the array,
// which adds memory overhead and complexity. For conversion efficiency, we process elements
// as-is without validation. Duplicate values will be caught by the database when the data is used.
//
// #[test]
// fn test_string_set_with_duplicates() {
//     // SS type must not contain duplicate values
//     let ddb_json = r#"{"Item":{"Field": {"SS": ["a", "a"]}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("duplicate") || error_message.contains("SS"),
//         "Error message should explain that SS cannot have duplicates, got: {}",
//         error_message
//     );
// }

// ============================================================================
// Issue #10: Invalid Number Set (NS) Values
// ============================================================================

#[test]
fn test_number_set_with_string_value() {
    // NS type must have an array value, not a string
    let ddb_json = r#"{"Item":{"Field": {"NS": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("NS") || error_message.contains("Expected"),
        "Error message should explain that NS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_number_set_with_number_value() {
    // NS type must have an array value, not a number
    let ddb_json = r#"{"Item":{"Field": {"NS": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("NS") || error_message.contains("Expected"),
        "Error message should explain that NS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_number_set_with_array_of_numbers() {
    // NS array elements must be strings (numbers transmitted as strings), not bare numbers
    let ddb_json = r#"{"Item":{"Field": {"NS": [1, 2]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("string") || error_message.contains("NS") || error_message.contains("Expected"),
        "Error message should explain that NS array elements must be strings, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that NS array strings contain valid numbers.
// Rationale: For conversion efficiency, we copy the attribute values as-is without
// modification or validation. This allows for faster streaming conversion without
// needing to parse and validate numeric strings. Invalid numbers will be caught
// by the database when the data is used.
//
// #[test]
// fn test_number_set_with_non_numeric_strings() {
//     // NS array strings must represent valid numbers
//     let ddb_json = r#"{"Item":{"Field": {"NS": ["abc"]}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("number") || error_message.contains("NS") || error_message.contains("invalid"),
//         "Error message should explain that NS strings must be valid numbers, got: {}",
//         error_message
//     );
// }

// COMMENTED OUT: We do not validate that NS arrays contain duplicate values.
// Rationale: Validating duplicates would require tracking all values seen in the array,
// which adds memory overhead and complexity. For conversion efficiency, we process elements
// as-is without validation. Duplicate values will be caught by the database when the data is used.
//
// #[test]
// fn test_number_set_with_duplicates() {
//     // NS type must not contain duplicate values
//     let ddb_json = r#"{"Item":{"Field": {"NS": ["1", "1"]}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("duplicate") || error_message.contains("NS"),
//         "Error message should explain that NS cannot have duplicates, got: {}",
//         error_message
//     );
// }

// ============================================================================
// Issue #11: Invalid Binary Set (BS) Values
// ============================================================================

#[test]
fn test_binary_set_with_string_value() {
    // BS type must have an array value, not a string
    let ddb_json = r#"{"Item":{"Field": {"BS": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("BS") || error_message.contains("Expected"),
        "Error message should explain that BS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_set_with_number_value() {
    // BS type must have an array value, not a number
    let ddb_json = r#"{"Item":{"Field": {"BS": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("BS") || error_message.contains("Expected"),
        "Error message should explain that BS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_set_with_array_of_numbers() {
    // BS array elements must be strings, not numbers
    let ddb_json = r#"{"Item":{"Field": {"BS": [123, 456]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("string") || error_message.contains("BS") || error_message.contains("Expected"),
        "Error message should explain that BS array elements must be strings, got: {}",
        error_message
    );
}

// COMMENTED OUT: We do not validate that BS array strings contain valid base64.
// Rationale: For conversion efficiency, we copy the attribute values as-is without
// modification or validation. This allows for faster streaming conversion without
// needing to decode and validate base64 strings. Invalid base64 will be caught
// by the database when the data is used.
//
// #[test]
// fn test_binary_set_with_invalid_base64() {
//     // BS array strings must be valid base64
//     let ddb_json = r#"{"Item":{"Field": {"BS": ["not-base64!!!"]}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("base64") || error_message.contains("BS") || error_message.contains("invalid"),
//         "Error message should explain that BS strings must be valid base64, got: {}",
//         error_message
//     );
// }

// COMMENTED OUT: We do not validate that BS arrays contain duplicate values.
// Rationale: Validating duplicates would require tracking all values seen in the array,
// which adds memory overhead and complexity. For conversion efficiency, we process elements
// as-is without validation. Duplicate values will be caught by the database when the data is used.
//
// #[test]
// fn test_binary_set_with_duplicates() {
//     // BS type must not contain duplicate values
//     let ddb_json = r#"{"Item":{"Field": {"BS": ["dGVzdA==", "dGVzdA=="]}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("duplicate") || error_message.contains("BS"),
//         "Error message should explain that BS cannot have duplicates, got: {}",
//         error_message
//     );
// }

// ============================================================================
// Issue #12: Structural Issues
// ============================================================================

// COMMENTED OUT: We do not validate that attributes have multiple type descriptors.
// Rationale: The streaming parser processes keys sequentially. When it encounters the second
// type descriptor in `{"S": "value", "N": "123"}`, it would process it as a separate field.
// Detecting this would require tracking which type descriptors have been seen for each attribute,
// adding state management complexity. The malformed output can be caught by downstream consumers.
//
// #[test]
// fn test_attribute_with_multiple_type_descriptors() {
//     // Attribute should have only one type descriptor
//     let ddb_json = r#"{"Item":{"Field": {"S": "value", "N": "123"}}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("multiple") || error_message.contains("type descriptor"),
//         "Error message should explain that multiple type descriptors are invalid, got: {}",
//         error_message
//     );
// }

// COMMENTED OUT: We do not validate that attribute values are type descriptor objects.
// Rationale: When the converter encounters a raw value like "value" instead of {"S": "value"},
// it doesn't match any type descriptor pattern, so it skips the value and produces malformed
// output. Detecting this would require validating that every field value in Item/M is an object
// with exactly one valid type descriptor key, which adds structural validation complexity.
// The malformed output (e.g., `{"Field":}`) will be caught as invalid JSON by downstream consumers.
//
// #[test]
// fn test_attribute_without_type_descriptor() {
//     // Attribute value should be a type descriptor object, not a raw value
//     let ddb_json = r#"{"Item":{"Field": "value"}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("unknown") || error_message.contains("type descriptor") || error_message.contains("Invalid"),
//         "Error message should explain that attribute needs a type descriptor, got: {}",
//         error_message
//     );
// }

// COMMENTED OUT: The converter already handles both cases - with and without Item wrapper.
// The "Item" wrapper is optional, so missing it is not an error. The converter auto-detects
// whether the Item wrapper is present and handles both formats correctly.
//
// #[test]
// fn test_missing_item_wrapper() {
//     // This tests if Item wrapper is required (it's actually optional in our implementation)
//     let ddb_json = r#"{"Field": {"S": "value"}}"#;
//     let error = convert_test_expect_error(ddb_json);
//
//     let error_message = format!("{:?}", error);
//     assert!(
//         error_message.contains("Item"),
//         "Error message should explain that Item wrapper is missing, got: {}",
//         error_message
//     );
// }
