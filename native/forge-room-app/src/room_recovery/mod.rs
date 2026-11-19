//! Non-hosting inspection, recent-source catalog and reviewed public Room restoration.
mod center;
mod storage;
pub(crate) use center::State;
pub(crate) use storage::Lease;

#[cfg(test)]
pub fn lease_for_test(
    root: Option<&std::path::Path>,
    path: &std::path::Path,
) -> Result<Option<Lease>, String> {
    storage::Store::lease_for(root, path)
}
