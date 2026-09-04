// An excluded package still resolves its own `crate::` paths against its own
// root, not the workspace's.
use crate::store::LegacyStore;

pub fn describe(store: LegacyStore) -> String {
    store.path
}
