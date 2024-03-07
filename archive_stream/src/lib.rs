pub mod archive;
pub mod archiver;
pub mod decode_accounts;
pub mod errors;
pub mod extract_snapshot;
pub mod loader;

pub use archive::*;
pub use archiver::*;
pub use decode_accounts::*;
pub use errors::*;
pub use extract_snapshot::*;
pub use loader::*;

use log::info;

// TODO: parallelize stream
pub fn stream_archived_accounts(source: String, callback: ArchiveCallback) -> anyhow::Result<()> {
    let mut loader = ArchiveLoader::new(source)?;
    info!("loader created");
    let mut archiver = Archiver::new(callback)?;
    info!("archiver created");

    info!("Iterating snapshot...");
    for append_vec in loader.iter() {
        archiver.extract_accounts(append_vec?)?
    }
    info!("Done!");
    // drop(archiver);

    Ok(())
}
