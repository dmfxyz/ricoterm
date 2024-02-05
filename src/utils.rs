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
