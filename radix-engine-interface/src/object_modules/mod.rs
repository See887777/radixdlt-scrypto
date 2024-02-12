#[cfg(feature = "radix_engine_fuzzing")]
use arbitrary::Arbitrary;
use radix_engine_common::{ManifestSbor, ScryptoSbor};
use radix_engine_interface::blueprints::resource::RoleAssignmentInit;

pub mod metadata;
pub mod role_assignment;
pub mod royalty;

#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Default, Debug, Clone, PartialEq, Eq, ScryptoSbor, ManifestSbor)]
pub struct ModuleConfig<T: Default> {
    pub init: T,
    pub roles: RoleAssignmentInit,
}
