//! Computes the override fingerprint (SHA-256 of the serialized override set,
//! or `None` if no overrides are configured).

use crate::Overrides;

pub(super) fn compute_override_fingerprint(overrides: &Overrides) -> Option<String> {
    let has_overrides = !overrides.must_link.is_empty()
        || !overrides.force_members.is_empty()
        || !overrides.rename_slices.is_empty()
        || !overrides.must_order.is_empty();
    if !has_overrides {
        return None;
    }

    use sha2::{Digest, Sha256};
    let json = serde_json::to_string(overrides).unwrap_or_default();
    Some(format!("{:x}", Sha256::digest(json.as_bytes())))
}
