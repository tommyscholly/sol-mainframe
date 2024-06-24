use byteorder::{ByteOrder, LittleEndian};

pub fn _vec_u64_to_byte_array(numbers: &Vec<u64>) -> Vec<u8> {
    let mut bytes = vec![0u8; numbers.len() * 8]; // Each u64 consists of 8 bytes
    for (i, &number) in numbers.iter().enumerate() {
        LittleEndian::write_u64(&mut bytes[i * 8..(i + 1) * 8], number);
    }
    bytes
}

pub fn strip_token(token: String) -> String {
    token
        .strip_prefix('"')
        .unwrap()
        .strip_suffix('"')
        .unwrap()
        .to_string()
}
