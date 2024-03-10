use solana_sdk::pubkey::Pubkey;

#[allow(dead_code)]
pub fn shorten_address(key: &Pubkey) -> String {
    // shorten address to 4 characters ... 4 characters
    let str = key.to_string();
    let first_4_chars = &str[0..4];
    let middle_3_dots = "...";
    let last_4_chars = &str[str.len() - 4..];
    format!("{}{}{}", first_4_chars, middle_3_dots, last_4_chars)
}
