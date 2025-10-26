#[test]
fn test_string_ddb_to_normal() {
    let ddb_json = r#"{"Item": {"name": {"S": "Alice"}}}"#;
    let mut reader = ddb_json.as_bytes();
    let mut output = vec![0u8; 1024];
    let mut output_slice = output.as_mut_slice();
    let mut rjiter_buffer = [0u8; 4096];
    let mut context_buffer = [0u8; 2048];

    ddb_convert::convert_ddb_to_normal(
        &mut reader,
        &mut output_slice,
        &mut rjiter_buffer,
        &mut context_buffer,
        false,
    ).unwrap();

    let bytes_written = 1024 - output_slice.len();
    let result = std::str::from_utf8(&output[..bytes_written]).unwrap();
    let expected = r#"{"name": "Alice"}"#;
    assert_eq!(result, expected);
}
