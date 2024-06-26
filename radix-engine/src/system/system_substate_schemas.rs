use crate::internal_prelude::*;
use crate::kernel::kernel::*;
use crate::system::system_callback::*;
use crate::vm::*;
use lazy_static::lazy_static;
use sbor::*;

use crate::system::type_info::TypeInfoSubstate;

// This is for having schemas to help map system substates
lazy_static! {
    pub static ref TYPE_INFO_SUBSTATE_SCHEMA: (LocalTypeId, VersionedScryptoSchema) =
        generate_full_schema_from_single_type::<TypeInfoSubstate, _>();
    pub static ref KERNEL_BOOT_SUBSTATE_SCHEMA: (LocalTypeId, VersionedScryptoSchema) =
        generate_full_schema_from_single_type::<KernelBootSubstate, _>();
    pub static ref SYSTEM_BOOT_SUBSTATE_SCHEMA: (LocalTypeId, VersionedScryptoSchema) =
        generate_full_schema_from_single_type::<SystemBootSubstate, _>();
    pub static ref VM_BOOT_SUBSTATE_SCHEMA: (LocalTypeId, VersionedScryptoSchema) =
        generate_full_schema_from_single_type::<VmBootSubstate, _>();
    pub static ref SCHEMA_SUBSTATE_SCHEMA: (LocalTypeId, VersionedScryptoSchema) =
        generate_full_schema_from_single_type::<PackageSchemaEntrySubstate, _>();
}
