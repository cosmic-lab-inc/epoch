pub trait DecodeProgramAccount: Sized {
    fn decode_account(discrim: &str, data: &[u8]) -> anyhow::Result<Self>;
}
