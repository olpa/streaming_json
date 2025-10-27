/// Helper function to convert DDB JSON to normal JSON for testing
fn convert_test(ddb_json: &str) -> String {
    convert_test_with_pretty(ddb_json, false)
}

/// Helper function to convert DDB JSON to normal JSON with optional pretty printing
fn convert_test_with_pretty(ddb_json: &str, pretty: bool) -> String {
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
        pretty,
    ).unwrap();

    let bytes_written = 4096 - output_slice.len();
    std::str::from_utf8(&output[..bytes_written]).unwrap().to_string()
}

#[test]
fn test_string_type() {
    let ddb_json = r#"{"Item":{"name": {"S": "Alice"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"name":"Alice"}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_number_type() {
    let ddb_json = r#"{"Item":{"age": {"N": "42"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"age":42}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_number_decimal() {
    let ddb_json = r#"{"Item":{"price": {"N": "3.14159"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"price":3.14159}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_boolean_true() {
    let ddb_json = r#"{"Item":{"active": {"BOOL": true}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"active":true}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_boolean_false() {
    let ddb_json = r#"{"Item":{"inactive": {"BOOL": false}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"inactive":false}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_null_type() {
    let ddb_json = r#"{"Item":{"empty": {"NULL": true}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"empty":null}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_string_set() {
    let ddb_json = r#"{"Item":{"tags": {"SS": ["apple", "banana", "cherry"]}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"tags":["apple","banana","cherry"]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_number_set() {
    let ddb_json = r#"{"Item":{"scores": {"NS": ["1", "2", "3", "5", "8"]}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"scores":[1,2,3,5,8]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_binary_type() {
    let ddb_json = r#"{"Item":{"data": {"B": "VGhpcyBpcyBiYXNlNjQ="}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":"VGhpcyBpcyBiYXNlNjQ="}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_binary_set() {
    let ddb_json = r#"{"Item":{"binaries": {"BS": ["Zmlyc3Q=", "c2Vjb25k", "dGhpcmQ="]}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"binaries":["Zmlyc3Q=","c2Vjb25k","dGhpcmQ="]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_list_type() {
    let ddb_json = r#"{"Item":{"items": {"L": [{"S": "string"}, {"N": "123"}, {"BOOL": true}]}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"items":["string",123,true]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_list_with_maps() {
    let ddb_json = r#"{"Item":{"users": {"L": [{"M": {"name": {"S": "Alice"}, "age": {"N": "30"}}}, {"M": {"name": {"S": "Bob"}, "age": {"N": "25"}}}]}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"users":[{"name":"Alice","age":30},{"name":"Bob","age":25}]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_nested_lists() {
    let ddb_json = r#"{"Item":{"nested": {"L": [{"L": [{"S": "a"}, {"S": "b"}]}, {"L": [{"N": "1"}, {"N": "2"}]}]}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"nested":[["a","b"],[1,2]]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_empty_list() {
    let ddb_json = r#"{"Item":{"empty": {"L": []}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"empty":[]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_map_type() {
    let ddb_json = r#"{"Item":{"metadata": {"M": {"key1": {"S": "value1"}, "key2": {"N": "999"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"metadata":{"key1":"value1","key2":999}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_nested_map() {
    let ddb_json = r#"{"Item":{"outer": {"M": {"inner": {"M": {"deep": {"S": "nested"}}}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"outer":{"inner":{"deep":"nested"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_map_with_mixed_types() {
    let ddb_json = r#"{"Item":{"data": {"M": {"str": {"S": "hello"}, "num": {"N": "123"}, "bool": {"BOOL": true}, "null": {"NULL": true}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"str":"hello","num":123,"bool":true,"null":null}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_empty_map() {
    let ddb_json = r#"{"Item":{"empty": {"M": {}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"empty":{}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_multiple_fields() {
    let ddb_json = r#"{"Item":{"name": {"S": "Bob"}, "age": {"N": "30"}, "active": {"BOOL": true}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"name":"Bob","age":30,"active":true}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_mixed_types() {
    let ddb_json = r#"{"Item":{
        "id":{"S": "test-001"},
        "count":{"N": "42"},
        "enabled":{"BOOL": false},
        "nothing":{"NULL": true},
        "tags":{"SS": ["tag1", "tag2"]}
    }}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"id":"test-001","count":42,"enabled":false,"nothing":null,"tags":["tag1","tag2"]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_empty_string_set() {
    let ddb_json = r#"{"Item":{"tags": {"SS": []}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"tags":[]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_empty_number_set() {
    let ddb_json = r#"{"Item":{"numbers": {"NS": []}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"numbers":[]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_empty_binary_set() {
    let ddb_json = r#"{"Item":{"binaries": {"BS": []}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"binaries":[]}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_large_number() {
    let ddb_json = r#"{"Item":{"bigNum": {"N": "123456789012345678901234567890"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"bigNum":123456789012345678901234567890}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_negative_number() {
    let ddb_json = r#"{"Item":{"temp": {"N": "-273.15"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"temp":-273.15}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_special_characters_in_string() {
    let ddb_json = r#"{"Item":{"message": {"S": "Hello \"World\"!\nNew line\tTab"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"message":"Hello \"World\"!\nNew line\tTab"}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_unicode_in_string() {
    let ddb_json = r#"{"Item":{"emoji": {"S": "🚀 Hello 世界"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"emoji":"🚀 Hello 世界"}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_empty_string() {
    let ddb_json = r#"{"Item":{"empty": {"S": ""}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"empty":""}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_zero_number() {
    let ddb_json = r#"{"Item":{"zero": {"N": "0"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"zero":0}
"#;
    assert_eq!(result, expected);
}

// Tests for confusing field names that match type descriptors
#[test]
fn test_field_named_M_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"M": {"S": "value"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"M":"value"}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_L_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"L": {"S": "value"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"L":"value"}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_S_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"S": {"S": "value"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"S":"value"}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_N_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"N": {"N": "123"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"N":123}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_BOOL_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"BOOL": {"BOOL": true}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"BOOL":true}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_NULL_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"NULL": {"NULL": true}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"NULL":null}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_SS_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"SS": {"SS": ["a", "b"]}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"SS":["a","b"]}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_field_named_Item_inside_M() {
    let ddb_json = r#"{"Item":{"data": {"M": {"Item": {"S": "value"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"data":{"Item":"value"}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_nested_M_fields() {
    let ddb_json = r#"{"Item":{"a": {"M": {"M": {"M": {"b": {"S": "c"}}}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"a":{"M":{"b":"c"}}}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_mixed_confusing_fields() {
    let ddb_json = r#"{"Item":{"test": {"M": {"M": {"S": "m"}, "L": {"L": [{"S": "l"}]}, "S": {"S": "s"}}}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"test":{"M":"m","L":["l"],"S":"s"}}
"#;
    assert_eq!(result, expected);
}

// Tests for optional Item wrapper
#[test]
fn test_no_item_wrapper_simple() {
    let ddb_json = r#"{"name":{"S": "Alice"}, "age": {"N": "30"}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"name":"Alice","age":30}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_with_item_wrapper_simple() {
    let ddb_json = r#"{"Item":{"name": {"S": "Alice"}, "age": {"N": "30"}}}"#;
    let result = convert_test(ddb_json);
    let expected = r#"{"name":"Alice","age":30}
"#;
    assert_eq!(result, expected);
}

#[test]
fn test_nested_object_pretty_indentation() {
    let ddb_json = r#"{"Item":{"name":{"S":"Test"},"settings":{"M":{"theme":{"S":"dark"},"notifications":{"M":{"email":{"BOOL":true},"push":{"BOOL":false}}}}}}}"#;
    let result = convert_test_with_pretty(ddb_json, true);
    let expected = r#"{
  "name":"Test",
  "settings":{
    "theme":"dark",
    "notifications":{
      "email":true,
      "push":false
    }
  }
}
"#;
    assert_eq!(result, expected);
}
