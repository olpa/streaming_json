/// Tests for invalid DynamoDB JSON format validation

/// Helper function to test that conversion fails with an error
fn convert_test_expect_error(ddb_json: &str) -> (ddb_convert::ConversionError, usize) {
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
        false,
        ddb_convert::ItemWrapperMode::AsWrapper,
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
    let ddb_json = r#"{"Item":{"Field": {"N": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("Expected string") || error_message.contains("N"),
        "Error message should explain that N type expects a string value, got: {}",
        error_message
    );
}

// DELETED: We do not validate that N type strings contain valid numbers.
// Rationale: For conversion efficiency, we copy the attribute value as-is without
// modification or validation.

//
#[test]
fn test_number_type_with_null_value() {
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
    let ddb_json = r#"{"Item":{"Field": {"NULL": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true")
            || error_message.contains("NULL")
            || error_message.contains("True"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

#[test]
fn test_null_type_with_string_value() {
    let ddb_json = r#"{"Item":{"Field": {"NULL": "true"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true")
            || error_message.contains("NULL")
            || error_message.contains("True"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

#[test]
fn test_null_type_with_number_value() {
    let ddb_json = r#"{"Item":{"Field": {"NULL": 1}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("true")
            || error_message.contains("NULL")
            || error_message.contains("True"),
        "Error message should explain that NULL type expects true value, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #7: Invalid Map (M) Values
// ============================================================================

#[test]
fn test_map_type_with_string_value() {
    let ddb_json = r#"{"Item":{"Field": {"M": "not-an-object"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object")
            || error_message.contains("M")
            || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_number_value() {
    let ddb_json = r#"{"Item":{"Field": {"M": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object")
            || error_message.contains("M")
            || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_boolean_value() {
    let ddb_json = r#"{"Item":{"Field": {"M": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object")
            || error_message.contains("M")
            || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_null_value() {
    let ddb_json = r#"{"Item":{"Field": {"M": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("object")
            || error_message.contains("M")
            || error_message.contains("Expected"),
        "Error message should explain that M type expects an object value, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_array_value() {
    let ddb_json = r#"{"Item":{"Field": {"M": [{"S": "value"}]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array") || error_message.contains("unexpected"),
        "Error message should explain that an unexpected array was found, got: {}",
        error_message
    );
}

#[test]
fn test_map_type_with_nested_missing_type_descriptor() {
    let ddb_json = r#"{"Item":{"Field": {"M": {"Nested": "value"}}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("atom")
            || error_message.contains("array")
            || error_message.contains("Invalid"),
        "Error message should indicate structural validation error, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #8: Invalid List (L) Values
// ============================================================================

#[test]
fn test_list_type_with_string_value() {
    let ddb_json = r#"{"Item":{"Field": {"L": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("L")
            || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_number_value() {
    let ddb_json = r#"{"Item":{"Field": {"L": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("L")
            || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_boolean_value() {
    let ddb_json = r#"{"Item":{"Field": {"L": true}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("L")
            || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_null_value() {
    let ddb_json = r#"{"Item":{"Field": {"L": null}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("L")
            || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_object_value() {
    let ddb_json = r#"{"Item":{"Field": {"L": {"S": "value"}}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("L")
            || error_message.contains("Expected"),
        "Error message should explain that L type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_list_type_with_elements_missing_type_descriptor() {
    let ddb_json = r#"{"Item":{"Field": {"L": ["raw-value"]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("atom")
            || error_message.contains("array")
            || error_message.contains("Invalid"),
        "Error message should indicate structural validation error, got: {}",
        error_message
    );
}

// ============================================================================
// Issue #9: Invalid String Set (SS) Values
// ============================================================================

#[test]
fn test_string_set_with_string_value() {
    let ddb_json = r#"{"Item":{"Field": {"SS": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("SS")
            || error_message.contains("Expected"),
        "Error message should explain that SS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_string_set_with_number_value() {
    let ddb_json = r#"{"Item":{"Field": {"SS": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("SS")
            || error_message.contains("Expected"),
        "Error message should explain that SS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_string_set_with_array_of_numbers() {
    let ddb_json = r#"{"Item":{"Field": {"SS": [123, 456]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("string")
            || error_message.contains("SS")
            || error_message.contains("Expected"),
        "Error message should explain that SS array elements must be strings, got: {}",
        error_message
    );
}

// DELETED: We do not validate that SS arrays contain duplicate values.

// ============================================================================
// Issue #10: Invalid Number Set (NS) Values
// ============================================================================

#[test]
fn test_number_set_with_string_value() {
    let ddb_json = r#"{"Item":{"Field": {"NS": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("NS")
            || error_message.contains("Expected"),
        "Error message should explain that NS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_number_set_with_number_value() {
    let ddb_json = r#"{"Item":{"Field": {"NS": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("NS")
            || error_message.contains("Expected"),
        "Error message should explain that NS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_number_set_with_array_of_numbers() {
    let ddb_json = r#"{"Item":{"Field": {"NS": [1, 2]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("string")
            || error_message.contains("NS")
            || error_message.contains("Expected"),
        "Error message should explain that NS array elements must be strings, got: {}",
        error_message
    );
}

// DELETED: We do not validate that NS array strings contain valid numbers.

// DELETED: We do not validate that NS arrays contain duplicate values.

// ============================================================================
// Issue #11: Invalid Binary Set (BS) Values
// ============================================================================

#[test]
fn test_binary_set_with_string_value() {
    let ddb_json = r#"{"Item":{"Field": {"BS": "not-an-array"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("BS")
            || error_message.contains("Expected"),
        "Error message should explain that BS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_set_with_number_value() {
    let ddb_json = r#"{"Item":{"Field": {"BS": 123}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("array")
            || error_message.contains("BS")
            || error_message.contains("Expected"),
        "Error message should explain that BS type expects an array value, got: {}",
        error_message
    );
}

#[test]
fn test_binary_set_with_array_of_numbers() {
    let ddb_json = r#"{"Item":{"Field": {"BS": [123, 456]}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("string")
            || error_message.contains("BS")
            || error_message.contains("Expected"),
        "Error message should explain that BS array elements must be strings, got: {}",
        error_message
    );
}

// DELETED: We do not validate that BS array strings contain valid base64.

// DELETED: We do not validate that BS arrays contain duplicate values.

// ============================================================================
// Issue #12: Structural Issues
// ============================================================================

#[test]
fn test_attribute_with_multiple_type_descriptors() {
    let ddb_json = r#"{"Item":{"Field": {"S": "value", "N": "123"}}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("atom")
            || error_message.contains("array")
            || error_message.contains("Invalid"),
        "Error message should indicate structural validation error, got: {}",
        error_message
    );
}

#[test]
fn test_attribute_without_type_descriptor() {
    let ddb_json = r#"{"Item":{"Field": "value"}}"#;
    let error = convert_test_expect_error(ddb_json);

    let error_message = format!("{:?}", error);
    assert!(
        error_message.contains("atom")
            || error_message.contains("array")
            || error_message.contains("Invalid"),
        "Error message should indicate structural validation error, got: {}",
        error_message
    );
}
