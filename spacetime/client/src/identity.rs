use solana_sdk::pubkey::Pubkey;
use spacetimedb_sdk::anyhow;
use spacetimedb_sdk::identity::Identity;

pub trait FromPubkey {
    fn from_pubkey(pubkey: &Pubkey) -> Identity;
}
impl FromPubkey for Identity {
    fn from_pubkey(pubkey: &Pubkey) -> Identity {
        Identity::from_bytes(pubkey.to_bytes().into())
    }
}

pub trait FromIdentity {
    fn from_identity(pubkey: &Identity) -> anyhow::Result<Pubkey>;
}
impl FromIdentity for Pubkey {
    fn from_identity(id: &Identity) -> anyhow::Result<Pubkey> {
        let bytes: [u8; 32] = id.bytes().try_into()?;
        Ok(Pubkey::new_from_array(bytes))
    }
}
