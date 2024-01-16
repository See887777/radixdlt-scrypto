use crate::blueprints::models::KeyValueEntryContentSource;
use crate::blueprints::package::*;
use crate::blueprints::pool::v1::constants::*;
use crate::blueprints::pool::v1::package::*;
use crate::internal_prelude::*;
use crate::system::attached_modules::role_assignment::*;
use crate::system::system_db_reader::{ObjectCollectionKey, SystemDatabaseReader};
use crate::track::{NodeStateUpdates, PartitionStateUpdates, StateUpdates};
use crate::vm::VmApi;
use crate::vm::{VmBoot, BOOT_LOADER_VM_PARTITION_NUM, BOOT_LOADER_VM_SUBSTATE_FIELD_KEY};
use radix_engine_common::constants::{BOOT_LOADER_STATE, CONSENSUS_MANAGER_PACKAGE};
use radix_engine_common::crypto::hash;
use radix_engine_common::prelude::ScopedTypeId;
use radix_engine_common::prelude::{scrypto_encode, ScryptoCustomTypeKind};
use radix_engine_common::types::SubstateKey;
use radix_engine_interface::api::ObjectModuleId;
use radix_engine_interface::blueprints::consensus_manager::*;
use radix_engine_interface::prelude::*;
use radix_engine_interface::types::CollectionDescriptor;
use radix_engine_store_interface::db_key_mapper::*;
use radix_engine_store_interface::interface::*;
use sbor::HasLatestVersion;
use sbor::{generate_full_schema, TypeAggregator};
use utils::indexmap;

pub fn generate_vm_boot_scrypto_minor_version_state_updates() -> StateUpdates {
    let substate = scrypto_encode(&VmBoot::V1 {
        scrypto_v1_minor_version: 1u64,
    })
    .unwrap();

    StateUpdates {
        by_node: indexmap!(
            BOOT_LOADER_STATE => NodeStateUpdates::Delta {
                by_partition: indexmap! {
                    BOOT_LOADER_VM_PARTITION_NUM => PartitionStateUpdates::Delta {
                        by_substate: indexmap! {
                            SubstateKey::Field(BOOT_LOADER_VM_SUBSTATE_FIELD_KEY) => DatabaseUpdate::Set(substate)
                        }
                    },
                }
            }
        ),
    }
}

/// Generates the state updates required for updating the Consensus Manager blueprint
/// to use seconds precision
pub fn generate_seconds_precision_state_updates<S: SubstateDatabase>(db: &S) -> StateUpdates {
    let reader = SystemDatabaseReader::new(db);
    let consensus_mgr_node_id = CONSENSUS_MANAGER_PACKAGE.into_node_id();
    let bp_version_key = BlueprintVersionKey {
        blueprint: CONSENSUS_MANAGER_BLUEPRINT.to_string(),
        version: BlueprintVersion::default(),
    };

    // Generate the new code substates
    let (new_code_substate, new_vm_type_substate, code_hash) = {
        let original_code = CONSENSUS_MANAGER_SECONDS_PRECISION_CODE_ID
            .to_be_bytes()
            .to_vec();

        let code_hash = CodeHash::from_hash(hash(&original_code));
        let versioned_code = VersionedPackageCodeOriginalCode::V1(PackageCodeOriginalCodeV1 {
            code: original_code,
        });
        let code_payload = versioned_code.into_payload();
        let code_substate = code_payload.into_locked_substate();
        let vm_type_substate = PackageCodeVmTypeV1 {
            vm_type: VmType::Native,
        }
        .into_versioned()
        .into_locked_substate();
        (
            scrypto_encode(&code_substate).unwrap(),
            scrypto_encode(&vm_type_substate).unwrap(),
            code_hash,
        )
    };

    // Generate the new schema substate
    let (
        new_schema_substate,
        get_current_time_input_v2_type_id,
        compare_current_time_input_v2_type_id,
        new_schema_hash,
    ) = {
        let mut aggregator = TypeAggregator::<ScryptoCustomTypeKind>::new();
        let get_current_time_input_v2 =
            aggregator.add_child_type_and_descendents::<ConsensusManagerGetCurrentTimeInputV2>();
        let compare_current_time_input_v2 = aggregator
            .add_child_type_and_descendents::<ConsensusManagerCompareCurrentTimeInputV2>();
        let schema = generate_full_schema(aggregator);
        let schema_hash = schema.generate_schema_hash();
        let schema_substate = schema.into_locked_substate();
        (
            scrypto_encode(&schema_substate).unwrap(),
            get_current_time_input_v2,
            compare_current_time_input_v2,
            schema_hash,
        )
    };

    // Generate the blueprint definition substate updates
    let updated_bp_definition_substate = {
        let versioned_definition: VersionedPackageBlueprintVersionDefinition = reader
            .read_object_collection_entry(
                &consensus_mgr_node_id,
                ObjectModuleId::Main,
                ObjectCollectionKey::KeyValue(
                    PackageCollection::BlueprintVersionDefinitionKeyValue.collection_index(),
                    &bp_version_key,
                ),
            )
            .unwrap()
            .unwrap();

        let mut definition = versioned_definition.into_latest();

        let export = definition
            .function_exports
            .get_mut(CONSENSUS_MANAGER_GET_CURRENT_TIME_IDENT)
            .unwrap();
        export.code_hash = code_hash;
        let function_schema = definition
            .interface
            .functions
            .get_mut(CONSENSUS_MANAGER_GET_CURRENT_TIME_IDENT)
            .unwrap();
        function_schema.input = BlueprintPayloadDef::Static(ScopedTypeId(
            new_schema_hash,
            get_current_time_input_v2_type_id,
        ));

        let export = definition
            .function_exports
            .get_mut(CONSENSUS_MANAGER_COMPARE_CURRENT_TIME_IDENT)
            .unwrap();
        export.code_hash = code_hash;
        let function_schema = definition
            .interface
            .functions
            .get_mut(CONSENSUS_MANAGER_COMPARE_CURRENT_TIME_IDENT)
            .unwrap();
        function_schema.input = BlueprintPayloadDef::Static(ScopedTypeId(
            new_schema_hash,
            compare_current_time_input_v2_type_id,
        ));

        scrypto_encode(
            &VersionedPackageBlueprintVersionDefinition::V1(definition).into_locked_substate(),
        )
        .unwrap()
    };

    let bp_definition_partition_num = reader
        .get_partition_of_collection(
            &consensus_mgr_node_id,
            ObjectModuleId::Main,
            PackageCollection::BlueprintVersionDefinitionKeyValue.collection_index(),
        )
        .unwrap();

    let code_vm_type_partition_num = reader
        .get_partition_of_collection(
            &consensus_mgr_node_id,
            ObjectModuleId::Main,
            PackageCollection::CodeVmTypeKeyValue.collection_index(),
        )
        .unwrap();

    let code_partition_num = reader
        .get_partition_of_collection(
            &consensus_mgr_node_id,
            ObjectModuleId::Main,
            PackageCollection::CodeOriginalCodeKeyValue.collection_index(),
        )
        .unwrap();

    let schema_partition_num = reader
        .get_partition_of_collection(
            &consensus_mgr_node_id,
            ObjectModuleId::Main,
            PackageCollection::SchemaKeyValue.collection_index(),
        )
        .unwrap();

    StateUpdates {
        by_node: indexmap!(
            consensus_mgr_node_id => NodeStateUpdates::Delta {
                by_partition: indexmap! {
                    bp_definition_partition_num => PartitionStateUpdates::Delta {
                        by_substate: indexmap! {
                            SubstateKey::Map(scrypto_encode(&bp_version_key).unwrap()) => DatabaseUpdate::Set(
                                updated_bp_definition_substate
                            )
                        }
                    },
                    code_vm_type_partition_num => PartitionStateUpdates::Delta {
                        by_substate: indexmap! {
                            SubstateKey::Map(scrypto_encode(&code_hash).unwrap()) => DatabaseUpdate::Set(new_vm_type_substate)
                        }
                    },
                    code_partition_num => PartitionStateUpdates::Delta {
                        by_substate: indexmap! {
                            SubstateKey::Map(scrypto_encode(&code_hash).unwrap()) => DatabaseUpdate::Set(new_code_substate)
                        }
                    },
                    schema_partition_num => PartitionStateUpdates::Delta {
                        by_substate: indexmap! {
                            SubstateKey::Map(scrypto_encode(&new_schema_hash).unwrap()) => DatabaseUpdate::Set(new_schema_substate)
                        }
                    }
                }
            }
        ),
    }
}

pub mod pools_package_v1_1 {
    use super::*;

    const PACKAGE_COLLECTIONS: [PackageCollection; 8] = [
        PackageCollection::BlueprintVersionDefinitionKeyValue,
        PackageCollection::BlueprintVersionDependenciesKeyValue,
        PackageCollection::SchemaKeyValue,
        PackageCollection::BlueprintVersionRoyaltyConfigKeyValue,
        PackageCollection::BlueprintVersionAuthConfigKeyValue,
        PackageCollection::CodeVmTypeKeyValue,
        PackageCollection::CodeOriginalCodeKeyValue,
        PackageCollection::CodeInstrumentedCodeKeyValue,
    ];

    pub fn generate_state_updates<S: SubstateDatabase + ListableSubstateDatabase>(
        db: &S,
    ) -> StateUpdates {
        let mut state_updates = StateUpdates::default();
        generate_package_state_updated(db, &mut state_updates);
        generate_role_assignment_update(db, &mut state_updates);
        state_updates
    }

    fn generate_package_state_updated<S: SubstateDatabase>(
        db: &S,
        state_updates: &mut StateUpdates,
    ) {
        let reader = SystemDatabaseReader::new(db);
        let node_id = POOL_PACKAGE.into_node_id();

        // Mark the entire partition of the existing collections for deletion
        for collection in PACKAGE_COLLECTIONS {
            let partition_num = reader
                .get_partition_of_collection(
                    &node_id,
                    ModuleId::Main,
                    collection.collection_index(),
                )
                .unwrap();

            state_updates
                .by_node
                .entry(node_id)
                .or_default()
                .of_partition(partition_num)
                .delete()
        }

        // Create the new substates based on the new definition and code id.
        let new_code_id = POOL_V1_1_CODE_ID;
        let original_code = new_code_id.to_be_bytes().to_vec();

        let package_structure = PackageNativePackage::validate_and_build_package_structure(
            PoolNativePackage::definition(PoolV1MinorVersion::One),
            VmType::Native,
            original_code,
            Default::default(),
            &MockVmApi,
        )
        .unwrap();

        let royalty_vault = reader
            .read_object_field(
                &node_id,
                ModuleId::Main,
                PackageField::RoyaltyAccumulator.field_index(),
            )
            .unwrap()
            .as_typed::<PackageRoyaltyAccumulatorFieldPayload>()
            .unwrap()
            .into_latest()
            .royalty_vault;

        let node_substates = create_package_partition_substates(
            package_structure,
            metadata_init! {
                "name" => "Pool Package".to_owned(), locked;
                "description" => "A native package that defines the logic for a selection of pool components.".to_owned(), locked;
            },
            Some(royalty_vault),
        );

        node_substates
            .into_iter()
            .for_each(|(partition_number, entries)| {
                state_updates
                    .by_node
                    .entry(node_id)
                    .or_default()
                    .of_partition(partition_number)
                    .update_substates(
                        entries
                            .into_iter()
                            .map(|(key, value)| (key, DatabaseUpdate::Set(value.into()))),
                    );
            })
    }

    /// Generates the state updates required for the new pool role.
    ///
    /// We have added a new role to the pool blueprints with v1.1 of the blueprints called the
    /// `pool_contributor` role, which is the role that is allowed to contribute assets to the pool.
    /// We would like pools that have already been instantiated to continue to function in the same
    /// way as it did before the protocol update. As in, the protocol manager was the only role that
    /// has contribution powers. So, for each pool that we find, we add a new role assignment for
    /// the contributor role with the same rule as the pool manager. Thus, pools that were created
    /// without this role will continue to function in the same way.
    fn generate_role_assignment_update<S: SubstateDatabase + ListableSubstateDatabase>(
        db: &S,
        state_updates: &mut StateUpdates,
    ) {
        let reader = SystemDatabaseReader::new(db);

        // Find all pools so that we apply this state changes to them
        let pools = reader.partitions_iter().filter_map(|(node_id, _)| {
            if node_id.entity_type().is_some_and(|entity_type| {
                matches!(
                    entity_type,
                    EntityType::GlobalOneResourcePool
                        | EntityType::GlobalTwoResourcePool
                        | EntityType::GlobalMultiResourcePool
                )
            }) {
                Some(node_id)
            } else {
                None
            }
        });

        let key = SubstateKey::Map(
            scrypto_encode(&ModuleRoleKey::new(ModuleId::Main, POOL_CONTRIBUTOR_ROLE)).unwrap(),
        );
        let mut already_seen = indexset! {};
        for node_id in pools {
            if already_seen.contains(&node_id) {
                continue;
            } else {
                already_seen.insert(node_id)
            };

            let partition_number = reader
                .get_partition_of_collection(
                    &node_id,
                    ModuleId::RoleAssignment,
                    RoleAssignmentCollection::AccessRuleKeyValue.collection_index(),
                )
                .unwrap();

            let substate_value = db
                .get_substate(
                    &SpreadPrefixKeyMapper::to_db_partition_key(&node_id, partition_number),
                    &SpreadPrefixKeyMapper::to_db_sort_key(&SubstateKey::Map(
                        scrypto_encode(&ModuleRoleKey::new(ModuleId::Main, POOL_MANAGER_ROLE))
                            .unwrap(),
                    )),
                )
                .unwrap();

            state_updates
                .of_node(node_id)
                .of_partition(partition_number)
                .update_substates([(key.clone(), DatabaseUpdate::Set(substate_value))])
        }
    }

    struct MockVmApi;

    impl VmApi for MockVmApi {
        fn get_scrypto_minor_version(&self) -> u64 {
            0
        }
    }
}
