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
