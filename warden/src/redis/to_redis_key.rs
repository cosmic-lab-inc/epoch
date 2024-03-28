use solana_sdk::pubkey::Pubkey;
use std::fmt::Debug;

macro_rules! display_redis_key {
    ($($t:ty),+ $(,)?) => {
        $(
            impl ToRedisKey for $t {
                fn to_redis_key(&self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}
display_redis_key!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, String);

/// Type can be a redis key
pub trait ToRedisKey: Debug {
    /// Converts this to a redis key
    fn to_redis_key(&self) -> String;
}

impl ToRedisKey for Pubkey {
    fn to_redis_key(&self) -> String {
        self.to_string()
    }
}
