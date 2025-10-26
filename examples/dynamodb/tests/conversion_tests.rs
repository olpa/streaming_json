use ddb_convert::convert_ddb_to_normal;

#[test]
fn test_string_ddb_to_normal() {
    let ddb_json = r#"{"Item": {"name": {"S": "Alice"}}}"#;
    let result = convert_ddb_to_normal(ddb_json, false).unwrap();
    let expected = r#"{"name": "Alice"}"#;
    assert_eq!(result, expected);
}
