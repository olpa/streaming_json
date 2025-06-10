use rjiter::RJiter;
use scan_json::idtransform::idtransform;

#[test]
fn idt_atomic_on_top() {
    let input = r#"
        null
        true false
        42 3.14
        "hello"
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut writer = Vec::new();

    //
    // Apply
    //
    idtransform(&mut rjiter, &mut writer); // null
    writer.push(b' ');
    idtransform(&mut rjiter, &mut writer); // true
    writer.push(b' ');
    idtransform(&mut rjiter, &mut writer); // false
    writer.push(b' ');
    idtransform(&mut rjiter, &mut writer); // 42
    writer.push(b' ');
    idtransform(&mut rjiter, &mut writer); // 3.14
    writer.push(b' ');
    idtransform(&mut rjiter, &mut writer); // "hello"

    //
    // Assert
    //
    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join(" ");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}
