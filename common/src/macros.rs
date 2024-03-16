#[macro_export]
macro_rules! trunc {
    ($num:expr, $decimals:expr) => {{
        let factor = 10.0_f64.powi($decimals);
        ($num * factor).round() / factor
    }};
}

#[macro_export]
macro_rules! decode_account {
    ($vis:vis enum $ident:ident {
        $($variant:ident ($account_type:ty)),*$(,)?
    }) => {
        #[derive(BorshDeserialize, BorshSerialize)]
        $vis enum $ident {
            $($variant($account_type),)*
        }

        impl DecodeProgramAccount for $ident {
            fn decode_account(discrim: &str, data: &[u8]) -> anyhow::Result<Self> {
                match discrim {
                    $(
                      $variant => Ok(Self::$variant(<$account_type>::try_from_slice(data)?)),
                    )*
                    _ => Err(anyhow::anyhow!("Invalid account discriminant")),
                }
            }
        }

    };
}
