use crate::archive::{AppendVecMeta, StoredAccountMetaHandle};
use std::rc::Rc;

pub type AppendVecIterator<'a> = Box<dyn Iterator<Item = anyhow::Result<AppendVecMeta>> + 'a>;

pub trait ArchiveIterator: Sized {
    fn iter(&mut self) -> AppendVecIterator<'_>;
}

pub fn append_vec_iter(meta: Rc<AppendVecMeta>) -> Vec<StoredAccountMetaHandle> {
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
    let meta = Rc::clone(&meta);
    let res: Vec<StoredAccountMetaHandle> = offsets
        .into_iter()
        .map(move |offset| StoredAccountMetaHandle::new(Rc::clone(&meta), offset))
        .collect();
    res
}
