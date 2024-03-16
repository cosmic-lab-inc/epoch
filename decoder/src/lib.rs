pub mod program_decoder;
pub mod program_helpers;

pub use program_decoder::*;
pub use program_helpers::*;

#[test]
fn test_base64_encoding() {
    use base64::{engine::general_purpose, Engine as _};
    let bytes: Vec<u8> = vec![159, 117, 95, 227, 239, 151, 58, 236];
    let data = general_purpose::STANDARD.encode(&bytes);
    println!("base64 data: {}", data);
}
