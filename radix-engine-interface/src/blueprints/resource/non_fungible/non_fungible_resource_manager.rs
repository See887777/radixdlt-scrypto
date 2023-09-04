use crate::blueprints::resource::*;
use crate::*;
#[cfg(feature = "radix_engine_fuzzing")]
use arbitrary::{Arbitrary, Result, Unstructured};
use radix_engine_common::data::manifest::model::ManifestAddressReservation;
use radix_engine_common::data::manifest::ManifestValue;
use radix_engine_common::data::scrypto::{
    ScryptoCustomTypeKind, ScryptoValue, VersionedScryptoSchema,
};
use radix_engine_common::prelude::replace_self_package_address;
use radix_engine_common::prelude::*;
use radix_engine_interface::api::node_modules::auth::ToRoleEntry;
use radix_engine_interface::api::node_modules::metadata::MetadataInit;
use radix_engine_interface::api::node_modules::ModuleConfig;
use radix_engine_interface::types::NonFungibleData;
use sbor::rust::collections::{IndexMap, IndexSet};
use sbor::rust::string::String;
use sbor::rust::string::ToString;
use sbor::rust::vec::Vec;
use sbor::{generate_full_schema, LocalTypeId, TypeAggregator};

pub const NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT: &str = "NonFungibleResourceManager";

#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Default, Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct NonFungibleResourceRoles {
    pub mint_roles: Option<MintRoles<RoleDefinition>>,
    pub burn_roles: Option<BurnRoles<RoleDefinition>>,
    pub freeze_roles: Option<FreezeRoles<RoleDefinition>>,
    pub recall_roles: Option<RecallRoles<RoleDefinition>>,
    pub withdraw_roles: Option<WithdrawRoles<RoleDefinition>>,
    pub deposit_roles: Option<DepositRoles<RoleDefinition>>,
    pub non_fungible_data_update_roles: Option<NonFungibleDataUpdateRoles<RoleDefinition>>,
}

impl NonFungibleResourceRoles {
    pub fn single_locked_rule(access_rule: AccessRule) -> Self {
        Self {
            mint_roles: mint_roles! {
                minter => access_rule.clone();
                minter_updater => rule!(deny_all);
            },
            burn_roles: burn_roles! {
                burner => access_rule.clone();
                burner_updater => rule!(deny_all);
            },
            freeze_roles: freeze_roles! {
                freezer => access_rule.clone();
                freezer_updater => rule!(deny_all);
            },
            recall_roles: recall_roles! {
                recaller => access_rule.clone();
                recaller_updater => rule!(deny_all);
            },
            non_fungible_data_update_roles: non_fungible_data_update_roles! {
                non_fungible_data_updater => access_rule.clone();
                non_fungible_data_updater_updater => rule!(deny_all);
            },
            withdraw_roles: withdraw_roles! {
                withdrawer => access_rule.clone();
                withdrawer_updater => rule!(deny_all);
            },
            deposit_roles: deposit_roles! {
                depositor => access_rule.clone();
                depositor_updater => rule!(deny_all);
            },
        }
    }
}

pub const NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_IDENT: &str = "create";

#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerCreateInput {
    pub owner_role: OwnerRole,
    pub id_type: NonFungibleIdType,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<GlobalAddressReservation>,
}

#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct NonFungibleResourceManagerCreateManifestInput {
    pub owner_role: OwnerRole,
    pub id_type: NonFungibleIdType,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<ManifestAddressReservation>,
}

pub type NonFungibleResourceManagerCreateOutput = ResourceAddress;

pub const NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_WITH_INITIAL_SUPPLY_IDENT: &str =
    "create_with_initial_supply";

#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerCreateWithInitialSupplyInput {
    pub owner_role: OwnerRole,
    pub id_type: NonFungibleIdType,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub entries: IndexMap<NonFungibleLocalId, (ScryptoValue,)>,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<GlobalAddressReservation>,
}

/// For manifest
#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct NonFungibleResourceManagerCreateWithInitialSupplyManifestInput {
    pub owner_role: OwnerRole,
    pub id_type: NonFungibleIdType,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub entries: IndexMap<NonFungibleLocalId, (ManifestValue,)>,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<ManifestAddressReservation>,
}

/// For typed value, to skip any codec
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerCreateWithInitialSupplyTypedInput<T> {
    pub owner_role: OwnerRole,
    pub id_type: NonFungibleIdType,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub entries: IndexMap<NonFungibleLocalId, (T,)>,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<GlobalAddressReservation>,
}

pub type NonFungibleResourceManagerCreateWithInitialSupplyOutput = (ResourceAddress, Bucket);

pub const NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_RUID_WITH_INITIAL_SUPPLY_IDENT: &str =
    "create_ruid_non_fungible_with_initial_supply";

#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerCreateRuidWithInitialSupplyInput {
    pub owner_role: OwnerRole,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub entries: Vec<(ScryptoValue,)>,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<GlobalAddressReservation>,
}

/// For manifest
#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct NonFungibleResourceManagerCreateRuidWithInitialSupplyManifestInput {
    pub owner_role: OwnerRole,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub entries: Vec<(ManifestValue,)>,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<ManifestAddressReservation>,
}

/// For typed value, to skip any codec
#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerCreateRuidWithInitialSupplyTypedInput<T> {
    pub owner_role: OwnerRole,
    pub track_total_supply: bool,
    pub non_fungible_schema: NonFungibleDataSchema,
    pub entries: Vec<(T,)>,
    pub resource_roles: NonFungibleResourceRoles,
    pub metadata: ModuleConfig<MetadataInit>,
    pub address_reservation: Option<GlobalAddressReservation>,
}

pub type NonFungibleResourceManagerCreateRuidWithInitialSupplyOutput = (ResourceAddress, Bucket);

pub const NON_FUNGIBLE_RESOURCE_MANAGER_UPDATE_DATA_IDENT: &str = "update_non_fungible_data";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerUpdateDataInput {
    pub id: NonFungibleLocalId,
    pub field_name: String,
    pub data: ScryptoValue,
}

/// For manifest
#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct NonFungibleResourceManagerUpdateDataManifestInput {
    pub id: NonFungibleLocalId,
    pub field_name: String,
    pub data: ManifestValue,
}

/// For typed value, to skip any codec
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerUpdateDataTypedInput<T> {
    pub id: NonFungibleLocalId,
    pub field_name: String,
    pub data: T,
}

pub type NonFungibleResourceManagerUpdateDataOutput = ();

pub const NON_FUNGIBLE_RESOURCE_MANAGER_EXISTS_IDENT: &str = "non_fungible_exists";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerExistsInput {
    pub id: NonFungibleLocalId,
}

pub type NonFungibleResourceManagerExistsOutput = bool;

pub const NON_FUNGIBLE_RESOURCE_MANAGER_GET_NON_FUNGIBLE_IDENT: &str = "get_non_fungible";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerGetNonFungibleInput {
    pub id: NonFungibleLocalId,
}

pub type NonFungibleResourceManagerGetNonFungibleOutput = ScryptoValue;

pub const NON_FUNGIBLE_RESOURCE_MANAGER_MINT_IDENT: &str = "mint";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerMintInput {
    pub entries: IndexMap<NonFungibleLocalId, (ScryptoValue,)>,
}

/// For manifest
#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct NonFungibleResourceManagerMintManifestInput {
    pub entries: IndexMap<NonFungibleLocalId, (ManifestValue,)>,
}

/// For typed value, to skip any codec
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerMintTypedInput<T> {
    pub entries: IndexMap<NonFungibleLocalId, (T,)>,
}

pub type NonFungibleResourceManagerMintOutput = Bucket;

pub const NON_FUNGIBLE_RESOURCE_MANAGER_MINT_RUID_IDENT: &str = "mint_ruid";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerMintRuidInput {
    pub entries: Vec<(ScryptoValue,)>,
}

/// For manifest
#[cfg_attr(feature = "radix_engine_fuzzing", derive(Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct NonFungibleResourceManagerMintRuidManifestInput {
    pub entries: Vec<(ManifestValue,)>,
}

/// For typed value, to skip any codec
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerMintRuidTypedInput<T> {
    pub entries: Vec<(T,)>,
}

pub type NonFungibleResourceManagerMintRuidOutput = Bucket;

pub const NON_FUNGIBLE_RESOURCE_MANAGER_MINT_SINGLE_RUID_IDENT: &str = "mint_single_ruid";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerMintSingleRuidInput {
    pub entry: ScryptoValue,
}

/// For typed value, to skip any codec
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct NonFungibleResourceManagerMintSingleRuidTypedInput<T> {
    pub entry: T,
}

pub type NonFungibleResourceManagerMintSingleRuidOutput = (Bucket, NonFungibleLocalId);

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor, ManifestSbor)]
pub enum NonFungibleDataSchema {
    // TODO: ignore this variant in Scrypto for smaller code size
    Local {
        schema: VersionedScryptoSchema,
        type_id: LocalTypeId,
        mutable_fields: IndexSet<String>,
    },
    Remote {
        package_address: PackageAddress,
        blueprint_name: String,
        type_name: String,
        mutable_fields: IndexSet<String>,
    },
}

impl NonFungibleData for () {
    const MUTABLE_FIELDS: &'static [&'static str] = &[];
}

impl NonFungibleDataSchema {
    /// Arguments:
    /// * [`package_address`] - The package address to use for resolving types defined within this package
    pub fn new_local<N: NonFungibleData>(package_address: PackageAddress) -> Self {
        let mut aggregator = TypeAggregator::<ScryptoCustomTypeKind>::new();
        let type_id = aggregator.add_child_type_and_descendents::<N>();
        let mut schema = generate_full_schema(aggregator);
        replace_self_package_address(&mut schema, package_address);
        Self::Local {
            schema,
            type_id,
            mutable_fields: N::MUTABLE_FIELDS.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn new_remote<T: AsRef<[S]>, S: AsRef<str>>(
        package_address: PackageAddress,
        blueprint_name: String,
        type_name: String,
        mutable_fields: T,
    ) -> Self {
        Self::Remote {
            package_address,
            blueprint_name,
            type_name,
            mutable_fields: mutable_fields
                .as_ref()
                .iter()
                .map(|s| s.as_ref().to_string())
                .collect(),
        }
    }
}

#[cfg(feature = "radix_engine_fuzzing")]
impl<'a> Arbitrary<'a> for NonFungibleDataSchema {
    // At the moment I see no smart method to derive Arbitrary for type Schema, which is part of
    // ScryptoSchema, therefore implementing arbitrary by hand.
    // TODO: Introduce a method that genearates NonFungibleDataSchema in a truly random manner
    fn arbitrary(_u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self::new_local::<()>(TEST_UTILS_PACKAGE))
    }
}
