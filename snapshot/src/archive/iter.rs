use crate::archive::{AppendVecMeta, StoredAccountMetaHandle};
use std::sync::Arc;

pub type AppendVecIterator<'a> = Box<dyn Iterator<Item = anyhow::Result<AppendVecMeta>> + 'a>;

pub trait ArchiveIterator: Sized {
    fn iter(&mut self) -> AppendVecIterator<'_>;
}

pub fn append_vec_iter(meta: Arc<AppendVecMeta>) -> Vec<StoredAccountMetaHandle> {
    let mut offsets = Vec::<usize>::new();
    let mut offset = 0usize;
    loop {
        match meta.append_vec.get_account(offset) {
            None => break,
            Some((_, next_offset)) => {
                offsets.push(offset);
                offset = next_offset;
            }
        }
    }
    // TODO: par iter if possible
    let res: Vec<StoredAccountMetaHandle> = offsets
        .into_iter()
        .map(move |offset| StoredAccountMetaHandle::new(Arc::clone(&meta), offset))
        .collect();
    res
}
