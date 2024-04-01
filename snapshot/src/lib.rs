pub mod archive;
pub mod archiver;
pub mod decode_accounts;
pub mod errors;
pub mod extract_snapshot;
pub mod loader;

pub use archive::*;
pub use archiver::*;
use common::ArchiveAccount;
use crossbeam_channel::Sender;
pub use decode_accounts::*;
pub use errors::*;
pub use extract_snapshot::*;
pub use loader::*;
use std::sync::Arc;

pub fn stream_archived_accounts(
    source: String,
    sender: Arc<Sender<ArchiveAccount>>,
) -> anyhow::Result<()> {
    let mut loader = ArchiveLoader::new(source)?;

    // TODO: parallelize stream with rayon ?
    for append_vec in loader.iter() {
        Archiver::extract_accounts(Arc::new(append_vec?), sender.clone())?
    }

    Ok(())
}
