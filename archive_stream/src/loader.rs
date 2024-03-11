use crate::archive::{AppendVecIterator, ArchiveIterator};
use crate::extract_snapshot::{ArchiveSnapshotExtractor, UnpackedSnapshotExtractor};
use bytes::{Buf, Bytes};
use futures::stream::StreamExt;
use futures::TryStreamExt;
use log::info;
use reqwest::Response;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncRead, BufReader, ReadBuf};
use tokio_stream::Stream;
// use futures_util::StreamExt;
// use tokio_stream::Stream;

/// Snapshot (archive) load options:
/// - file, a compressed tarball with extension .tar.zst
/// - unpacked, the uncompressed snapshot .tar.zst file
/// - streamed from HTTP endpoint
pub enum ArchiveLoader {
    Unpacked(UnpackedSnapshotExtractor),
    ArchiveFile(ArchiveSnapshotExtractor<BufReader<File>>),
    ArchiveDownload(ArchiveSnapshotExtractor<BufReader<Response>>),
}

/// Load a snapshot from a file or HTTP stream
impl ArchiveLoader {
    pub async fn new(source: String) -> anyhow::Result<Self> {
        if source.starts_with("http://") || source.starts_with("https://") {
            ArchiveLoader::new_download(source).await
        } else {
            ArchiveLoader::new_file(source.as_ref())
                .await
                .map_err(Into::into)
        }
    }

    async fn new_download(url: String) -> anyhow::Result<ArchiveLoader> {
        let resp = reqwest::get(url).await?;

        // compute number of Gb in resp
        let len = resp.content_length().unwrap_or(0);
        let len_gb = len / 1024 / 1024 / 1024;
        info!("Stream snapshot from HTTP ({} Gb)", len_gb);

        let src = resp.bytes().await?;
        let loader = ArchiveSnapshotExtractor::from_reader(src).await?;
        Ok(ArchiveLoader::ArchiveDownload(loader))
    }

    async fn new_file(path: &Path) -> anyhow::Result<ArchiveLoader> {
        Ok(if path.is_dir() {
            ArchiveLoader::Unpacked(UnpackedSnapshotExtractor::open(path)?)
        } else {
            ArchiveLoader::ArchiveFile(ArchiveSnapshotExtractor::open(path).await?)
        })
    }
}

impl ArchiveIterator for ArchiveLoader {
    fn iter(&mut self) -> AppendVecIterator {
        match self {
            ArchiveLoader::Unpacked(loader) => loader.iter(),
            ArchiveLoader::ArchiveFile(loader) => loader.iter(),
            ArchiveLoader::ArchiveDownload(loader) => loader.iter(),
        }
    }
}
