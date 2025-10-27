/// Helper function to convert normal JSON to DDB JSON for testing
fn convert_to_ddb_test(normal_json: &str, with_item_wrapper: bool) -> String {
    convert_to_ddb_test_with_pretty(normal_json, with_item_wrapper, false)
}

/// Helper function to convert normal JSON to DDB JSON with optional pretty printing
fn convert_to_ddb_test_with_pretty(normal_json: &str, with_item_wrapper: bool, pretty: bool) -> String {
    let mut reader = normal_json.as_bytes();
    let mut output = vec![0u8; 8192];
    let mut output_slice = output.as_mut_slice();
    let mut rjiter_buffer = [0u8; 4096];
    let mut context_buffer = [0u8; 2048];

    ddb_convert::convert_normal_to_ddb(
        &mut reader,
        &mut output_slice,
        &mut rjiter_buffer,
        &mut context_buffer,
        pretty,
        with_item_wrapper,
    ).unwrap();

    let bytes_written = 8192 - output_slice.len();
    std::str::from_utf8(&output[..bytes_written]).unwrap().to_string()
}

#[test]
fn test_to_ddb_string() {
    let normal_json = r#"{"name": "Alice"}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"name":{"S":"Alice"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_number_integer() {
    let normal_json = r#"{"age": 42}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"age":{"N":"42"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_number_float() {
    let normal_json = r#"{"price": 3.14159}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"price":{"N":"3.14159"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_boolean_true() {
    let normal_json = r#"{"active": true}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"active":{"BOOL":true}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_boolean_false() {
    let normal_json = r#"{"inactive": false}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"inactive":{"BOOL":false}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_null() {
    let normal_json = r#"{"empty": null}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"empty":{"NULL":true}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_array_strings() {
    let normal_json = r#"{"tags": ["apple", "banana", "cherry"]}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"tags":{"L":[{"S":"apple"},{"S":"banana"},{"S":"cherry"}]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_array_numbers() {
    let normal_json = r#"{"scores": [1, 2, 3, 5, 8]}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"scores":{"L":[{"N":"1"},{"N":"2"},{"N":"3"},{"N":"5"},{"N":"8"}]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_array_mixed() {
    let normal_json = r#"{"items": ["string", 123, true, null]}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"items":{"L":[{"S":"string"},{"N":"123"},{"BOOL":true},{"NULL":true}]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_empty_array() {
    let normal_json = r#"{"empty": []}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"empty":{"L":[]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_object() {
    let normal_json = r#"{"metadata": {"key1": "value1", "key2": 999}}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"metadata":{"M":{"key1":{"S":"value1"},"key2":{"N":"999"}}}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_empty_object() {
    let normal_json = r#"{"empty": {}}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"empty":{"M":{}}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_arrays() {
    let normal_json = r#"{"nested": [["a", "b"], [1, 2]]}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"nested":{"L":[{"L":[{"S":"a"},{"S":"b"}]},{"L":[{"N":"1"},{"N":"2"}]}]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_array_with_objects() {
    let normal_json = r#"{"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"users":{"L":[{"M":{"name":{"S":"Alice"},"age":{"N":"30"}}},{"M":{"name":{"S":"Bob"},"age":{"N":"25"}}}]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_deeply_nested() {
    let normal_json = r#"{"outer": {"inner": {"deep": "nested"}}}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"outer":{"M":{"inner":{"M":{"deep":{"S":"nested"}}}}}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_multiple_fields() {
    let normal_json = r#"{"name": "Bob", "age": 30, "active": true}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"name":{"S":"Bob"},"age":{"N":"30"},"active":{"BOOL":true}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_all_types() {
    let normal_json = r#"{"id": "test-001", "count": 42, "enabled": false, "nothing": null, "tags": ["tag1", "tag2"]}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"id":{"S":"test-001"},"count":{"N":"42"},"enabled":{"BOOL":false},"nothing":{"NULL":true},"tags":{"L":[{"S":"tag1"},{"S":"tag2"}]}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_without_item_wrapper() {
    let normal_json = r#"{"name": "Alice", "age": 30}"#;
    let result = convert_to_ddb_test(normal_json, false);
    let expected = r#"{"name":{"S":"Alice"},"age":{"N":"30"}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_special_characters() {
    let normal_json = r#"{"message": "Hello \"World\"!\nNew line\tTab"}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"message":{"S":"Hello \"World\"!\nNew line\tTab"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_unicode() {
    let normal_json = r#"{"emoji": "🚀 Hello 世界"}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"emoji":{"S":"🚀 Hello 世界"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_empty_string() {
    let normal_json = r#"{"empty": ""}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"empty":{"S":""}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_zero_number() {
    let normal_json = r#"{"zero": 0}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"zero":{"N":"0"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_negative_number() {
    let normal_json = r#"{"temp": -273.15}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"temp":{"N":"-273.15"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_large_number() {
    let normal_json = r#"{"bigNum": 123456789012345678901234567890}"#;
    let result = convert_to_ddb_test(normal_json, true);
    let expected = r#"{"Item":{"bigNum":{"N":"123456789012345678901234567890"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_to_ddb_nested_object_pretty_indentation() {
    let normal_json = r#"{"name":"Test","settings":{"theme":"dark","notifications":{"email":true,"push":false}}}"#;
    let result = convert_to_ddb_test_with_pretty(normal_json, true, true);
    let expected = r#"{
  "Item":{
    "name":{
      "S":"Test"
    },
    "settings":{
      "M":{
        "theme":{
          "S":"dark"
        },
        "notifications":{
          "M":{
            "email":{
              "BOOL":true
            },
            "push":{
              "BOOL":false
            }
          }
        }
      }
    }
  }
}
"#;
    assert_eq!(result, expected);
}
