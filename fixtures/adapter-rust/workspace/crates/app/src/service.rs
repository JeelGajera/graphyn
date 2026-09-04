// Hyphen normalization: the package is `core-lib`, the path segment is
// `core_lib`. Also a re-export hop: `models::UserPayload` is declared in
// `models::payload`.
use core_lib::models::UserPayload;

// The same package reached under a dependency rename.
use aliased_core::models::payload::UserPayload as Renamed;

// A `[lib] name` that differs from the package name.
use kernel::Kernel;

pub struct Service;

impl Service {
    pub fn handle(&self, payload: UserPayload) -> String {
        payload.user_id.clone()
    }

    pub fn renamed(&self, other: Renamed) -> String {
        other.email.clone()
    }

    pub fn kernel_id(&self, k: Kernel) -> u32 {
        k.id
    }
}
