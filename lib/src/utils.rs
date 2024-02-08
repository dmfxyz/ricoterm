use ethers::types::H256;

pub fn string_to_bytes32(input: &str) -> H256 {
    // Convert the input string to bytes and ensure it's no longer than 32 bytes
    let mut bytes = input.as_bytes()[..std::cmp::min(input.len(), 32)].to_vec();
    // Pad the bytes array with zeros if less than 32 bytes
    while bytes.len() < 32 {
        bytes.push(0);
    }
    // Convert the bytes array to a fixed-size array
    let fixed_bytes: [u8; 32] = bytes.try_into().expect("slice with incorrect length");
    // Create H256 from the fixed-size array
    H256::from(fixed_bytes)
}

pub fn bytes32_to_string(input: H256) -> String {
    // Convert the H256 to a fixed-size array
    let bytes: [u8; 32] = input.into();
    // Convert the fixed-size array to a vector
    let mut bytes_vec = bytes.to_vec();
    // Remove trailing zeros
    while let Some(0) = bytes_vec.last() {
        bytes_vec.pop();
    }
    // Convert the vector to a string
    String::from_utf8(bytes_vec).unwrap()
}
