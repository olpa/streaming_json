use rjiter::LinePosition;
use rjiter::RJiter;
use std::io::Cursor;

#[test]
fn index_in_error() {
    let token_pos = 32;
    let lot_of_spaces = " ".repeat(token_pos);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_bool();
    let err = result.unwrap_err();
    assert_eq!(err.index, token_pos);
}

#[test]
fn position_for_error() {
    let leading_text = "\n \n  \n   \n    \n      \n   ";
    let input = format!(r#"{leading_text}null null"#);
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_str();
    match result {
        Err(rjiter_err) => {
            let pos = rjiter_err.get_position(&rjiter);
            // for column: jiter reports the position of the space after "null"
            // 3 spaces in `leading_text`, 4 characters in "null"
            assert_eq!(pos, LinePosition::new(7, 8));
        }
        _ => panic!("Expected JiterError"),
    }
}

#[test]
fn description_of_error() {
    let leading_text = "\n \n  \n   \n    \n      \n   ";
    let input = format!(r#"{leading_text}null null"#);
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_str();
    match result {
        Err(rjiter_err) => {
            // see `position_for_error` for the position
            let desc = rjiter_err.description(&rjiter);
            assert_eq!(desc, "expected string but found null at line 7 column 8");
        }
        _ => panic!("Expected JiterError"),
    }
}

#[test]
fn display_of_error() {
    let leading_text = "\n \n  \n   \n    \n      \n   ";
    let input = format!(r#"{leading_text}null null"#);
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_str();
    match result {
        Err(rjiter_err) => {
            let desc = format!("{rjiter_err}");
            assert_eq!(desc, "expected string but found null at index 31");
        }
        _ => panic!("Expected JiterError"),
    }
}
