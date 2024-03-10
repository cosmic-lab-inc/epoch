pub mod account;
pub mod archive;
pub mod archiver;
pub mod decode_accounts;
pub mod errors;
pub mod extract_snapshot;
pub mod loader;
pub mod utils;

pub use account::*;
pub use archive::*;
pub use archiver::*;
pub use decode_accounts::*;
pub use errors::*;
pub use extract_snapshot::*;
pub use loader::*;
use std::sync::Arc;
pub use utils::*;

pub async fn stream_archived_accounts(
    source: String,
    listener: &'static dyn AccountCallback,
) -> anyhow::Result<()> {
    let mut loader = ArchiveLoader::new(source)?;

    // TODO: parallelize stream
    for append_vec in loader.iter() {
        Archiver::extract_accounts(Arc::new(append_vec?), listener).await?
    }

    Ok(())
}
