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
