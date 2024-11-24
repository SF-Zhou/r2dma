pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    const TABLE: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let mut str = String::with_capacity(bytes.len() * 2 + bytes.len() / 4);
    for (idx, &byte) in bytes.iter().enumerate() {
        if idx % 4 == 0 && idx != 0 {
            str.push('-');
        }
        str.push(TABLE[(byte / 16) as usize]);
        str.push(TABLE[(byte % 16) as usize]);
    }
    str
}

#[cfg(test)]
mod tests {
    use super::bytes_to_hex_string;

    #[test]
    fn test_bytes_to_hex_string() {
        for i in u16::MIN..=u16::MAX {
            let a = format!("{:04x}", i);
            let b = bytes_to_hex_string(&i.to_be_bytes());
            assert_eq!(a, b);
        }

        assert_eq!(
            bytes_to_hex_string(&[0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8]),
            "00010203-04050607".to_owned()
        );
    }
}
