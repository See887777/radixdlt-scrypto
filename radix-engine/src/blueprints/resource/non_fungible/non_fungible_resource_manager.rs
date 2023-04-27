use crate::blueprints::resource::*;
use crate::errors::ApplicationError;
use crate::errors::RuntimeError;
use crate::kernel::kernel_api::{KernelNodeApi, KernelSubstateApi};
use crate::types::*;
use native_sdk::resource::ResourceManager;
use native_sdk::runtime::Runtime;
use radix_engine_interface::api::substate_lock_api::LockFlags;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::math::Decimal;
use radix_engine_interface::schema::KeyValueStoreSchema;
use radix_engine_interface::types::{NodeId, ResourceManagerOffset};
use radix_engine_interface::*;
use sbor::rust::borrow::Cow;

/// Represents an error when accessing a bucket.
#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum NonFungibleResourceManagerError {
    NonFungibleAlreadyExists(Box<NonFungibleGlobalId>),
    NonFungibleNotFound(Box<NonFungibleGlobalId>),
    InvalidField(String),
    FieldNotMutable(String),
    NonFungibleIdTypeDoesNotMatch(NonFungibleIdType, NonFungibleIdType),
    InvalidNonFungibleIdType,
    NonFungibleLocalIdProvidedForUUIDType,
    DropNonEmptyBucket,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct NonFungibleResourceManagerSubstate {
    pub total_supply: Decimal,
    pub id_type: NonFungibleIdType,
    pub non_fungible_type_index: LocalTypeIndex,
    pub non_fungible_table: Own,
    pub mutable_fields: BTreeSet<String>, // TODO: Integrate with KeyValueStore schema check?
}

fn build_non_fungible_resource_manager_substate<Y>(
    id_type: NonFungibleIdType,
    supply: usize,
    non_fungible_schema: NonFungibleDataSchema,
    api: &mut Y,
) -> Result<(NonFungibleResourceManagerSubstate, NodeId), RuntimeError>
where
    Y: ClientApi<RuntimeError>,
{
    let mut aggregator = TypeAggregator::<ScryptoCustomTypeKind>::new();
    let non_fungible_type = aggregator.add_child_type_and_descendents::<NonFungibleLocalId>();
    let key_schema = generate_full_schema::<ScryptoCustomTypeExtension>(aggregator);

    let mut kv_schema = non_fungible_schema.schema;

    // Key
    kv_schema.type_kinds.extend(key_schema.type_kinds);

    // Optional Value
    {
        let mut variants = BTreeMap::new();
        variants.insert(OPTION_VARIANT_NONE, vec![]);
        variants.insert(OPTION_VARIANT_SOME, vec![non_fungible_schema.non_fungible]);
        let type_kind = TypeKind::Enum { variants };
        kv_schema.type_kinds.push(type_kind);
    }

    // Key
    kv_schema.type_metadata.extend(key_schema.type_metadata);

    // Optional value
    {
        let metadata = TypeMetadata {
            type_name: Some(Cow::Borrowed("Option")),
            child_names: Some(ChildNames::EnumVariants(btreemap!(
                OPTION_VARIANT_NONE => TypeMetadata::no_child_names("None"),
                OPTION_VARIANT_SOME => TypeMetadata::no_child_names("Some"),
            ))),
        };
        kv_schema.type_metadata.push(metadata);
    }

    // Key
    kv_schema
        .type_validations
        .extend(key_schema.type_validations);

    // Optional value
    kv_schema.type_validations.push(TypeValidation::None);
    let value_index = LocalTypeIndex::SchemaLocalIndex(kv_schema.type_validations.len() - 1);

    let kv_schema = KeyValueStoreSchema {
        schema: kv_schema,
        key: non_fungible_type,
        value: value_index,
        can_own: false, // Only allow NonFungibles to store data/references
    };

    let nf_store_id = api.new_key_value_store(kv_schema)?;

    let resource_manager = NonFungibleResourceManagerSubstate {
        id_type,
        non_fungible_type_index: non_fungible_schema.non_fungible,
        total_supply: supply.into(),
        non_fungible_table: Own(nf_store_id),
        mutable_fields: non_fungible_schema.mutable_fields,
    };

    Ok((resource_manager, nf_store_id))
}

fn create_non_fungibles<Y>(
    resource_address: ResourceAddress,
    id_type: NonFungibleIdType,
    nf_store_id: NodeId,
    entries: BTreeMap<NonFungibleLocalId, ScryptoValue>,
    check_non_existence: bool,
    api: &mut Y,
) -> Result<(), RuntimeError>
where
    Y: ClientApi<RuntimeError>,
{
    let mut ids = BTreeSet::new();
    for (non_fungible_local_id, value) in entries {
        if non_fungible_local_id.id_type() != id_type {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::NonFungibleIdTypeDoesNotMatch(
                        non_fungible_local_id.id_type(),
                        id_type,
                    ),
                ),
            ));
        }

        let non_fungible_handle = api.lock_key_value_store_entry(
            &nf_store_id,
            &non_fungible_local_id.to_key(),
            LockFlags::MUTABLE,
        )?;

        if check_non_existence {
            let cur_non_fungible: Option<ScryptoValue> =
                api.sys_read_substate_typed(non_fungible_handle)?;

            if let Some(..) = cur_non_fungible {
                return Err(RuntimeError::ApplicationError(
                    ApplicationError::NonFungibleResourceManagerError(
                        NonFungibleResourceManagerError::NonFungibleAlreadyExists(Box::new(
                            NonFungibleGlobalId::new(resource_address, non_fungible_local_id),
                        )),
                    ),
                ));
            }
        }

        // TODO: Change interface so that we accept Option instead
        api.sys_write_substate(non_fungible_handle, scrypto_encode(&Some(value)).unwrap())?;
        api.sys_drop_lock(non_fungible_handle)?;
        ids.insert(non_fungible_local_id);
    }

    Ok(())
}

pub struct NonFungibleResourceManagerBlueprint;

impl NonFungibleResourceManagerBlueprint {
    pub(crate) fn create<Y>(
        id_type: NonFungibleIdType,
        non_fungible_schema: NonFungibleDataSchema,
        metadata: BTreeMap<String, String>,
        access_rules: BTreeMap<ResourceMethodAuthKey, (AccessRule, AccessRule)>,
        api: &mut Y,
    ) -> Result<ResourceAddress, RuntimeError>
    where
        Y: KernelNodeApi + ClientApi<RuntimeError>,
    {
        let global_node_id = api.kernel_allocate_node_id(EntityType::GlobalNonFungibleResource)?;
        let resource_address = ResourceAddress::new_or_panic(global_node_id.into());
        Self::create_with_address(
            id_type,
            non_fungible_schema,
            metadata,
            access_rules,
            resource_address.into(),
            api,
        )
    }

    pub(crate) fn create_with_address<Y>(
        id_type: NonFungibleIdType,
        non_fungible_schema: NonFungibleDataSchema,
        metadata: BTreeMap<String, String>,
        access_rules: BTreeMap<ResourceMethodAuthKey, (AccessRule, AccessRule)>,
        resource_address: [u8; NodeId::LENGTH], // TODO: Clean this up
        api: &mut Y,
    ) -> Result<ResourceAddress, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        // If address isn't user frame allocated or pre_allocated then
        // using this node_id will fail on create_node below
        let (resource_manager_substate, _) =
            build_non_fungible_resource_manager_substate(id_type, 0, non_fungible_schema, api)?;

        let object_id = api.new_object(
            NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
            vec![scrypto_encode(&resource_manager_substate).unwrap()],
        )?;

        let resource_address = ResourceAddress::new_or_panic(resource_address);
        globalize_resource_manager(object_id, resource_address, access_rules, metadata, api)?;

        Ok(resource_address)
    }

    pub(crate) fn create_with_initial_supply<Y>(
        id_type: NonFungibleIdType,
        non_fungible_schema: NonFungibleDataSchema,
        metadata: BTreeMap<String, String>,
        access_rules: BTreeMap<ResourceMethodAuthKey, (AccessRule, AccessRule)>,
        entries: BTreeMap<NonFungibleLocalId, (ScryptoValue,)>,
        api: &mut Y,
    ) -> Result<(ResourceAddress, Bucket), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        // TODO: Do this check in a better way (e.g. via type check)
        if id_type == NonFungibleIdType::UUID {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::NonFungibleLocalIdProvidedForUUIDType,
                ),
            ));
        }

        let (resource_manager, nf_store_id) = build_non_fungible_resource_manager_substate(
            id_type,
            entries.len(),
            non_fungible_schema,
            api,
        )?;

        let global_node_id = api.kernel_allocate_node_id(EntityType::GlobalNonFungibleResource)?;
        let resource_address = ResourceAddress::new_or_panic(global_node_id.into());

        let ids = entries.keys().cloned().collect();
        let non_fungibles = entries
            .into_iter()
            .map(|(id, (value,))| (id, value))
            .collect();
        create_non_fungibles(
            resource_address,
            id_type,
            nf_store_id,
            non_fungibles,
            false,
            api,
        )?;

        let object_id = api.new_object(
            NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
            vec![scrypto_encode(&resource_manager).unwrap()],
        )?;
        globalize_resource_manager(object_id, resource_address, access_rules, metadata, api)?;

        let bucket = ResourceManager(resource_address).new_non_fungible_bucket(ids, api)?;

        Ok((resource_address, bucket))
    }

    pub(crate) fn create_uuid_with_initial_supply<Y>(
        non_fungible_schema: NonFungibleDataSchema,
        metadata: BTreeMap<String, String>,
        access_rules: BTreeMap<ResourceMethodAuthKey, (AccessRule, AccessRule)>,
        entries: Vec<(ScryptoValue,)>,
        api: &mut Y,
    ) -> Result<(ResourceAddress, Bucket), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let mut ids = BTreeSet::new();
        let mut non_fungibles = BTreeMap::new();
        for (entry,) in entries {
            let uuid = Runtime::generate_uuid(api)?;
            let id = NonFungibleLocalId::uuid(uuid).unwrap();
            ids.insert(id.clone());
            non_fungibles.insert(id, entry);
        }

        let (resource_manager, nf_store_id) = build_non_fungible_resource_manager_substate(
            NonFungibleIdType::UUID,
            non_fungibles.len(),
            non_fungible_schema,
            api,
        )?;

        let global_node_id = api.kernel_allocate_node_id(EntityType::GlobalNonFungibleResource)?;
        let resource_address = ResourceAddress::new_or_panic(global_node_id.into());

        create_non_fungibles(
            resource_address,
            NonFungibleIdType::UUID,
            nf_store_id,
            non_fungibles,
            false,
            api,
        )?;

        let object_id = api.new_object(
            NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
            vec![scrypto_encode(&resource_manager).unwrap()],
        )?;
        globalize_resource_manager(object_id, resource_address, access_rules, metadata, api)?;

        let bucket = ResourceManager(resource_address).new_non_fungible_bucket(ids, api)?;

        Ok((resource_address, bucket))
    }

    pub(crate) fn mint_non_fungible<Y>(
        entries: BTreeMap<NonFungibleLocalId, (ScryptoValue,)>,
        api: &mut Y,
    ) -> Result<Bucket, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());

        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::MUTABLE,
        )?;
        let mut resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;

        let id_type = resource_manager.id_type.clone();
        if id_type == NonFungibleIdType::UUID {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::NonFungibleLocalIdProvidedForUUIDType,
                ),
            ));
        }
        resource_manager.total_supply += entries.len();
        let nf_store_id = resource_manager.non_fungible_table.as_node_id().clone();
        api.sys_write_substate_typed(resman_handle, resource_manager)?;

        let ids: BTreeSet<NonFungibleLocalId> = entries.keys().cloned().collect();
        let non_fungibles = entries.into_iter().map(|(k, v)| (k, v.0)).collect();
        create_non_fungibles(
            resource_address,
            id_type,
            nf_store_id,
            non_fungibles,
            true,
            api,
        )?;

        api.sys_drop_lock(resman_handle)?;

        let bucket = ResourceManager(resource_address).new_non_fungible_bucket(ids.clone(), api)?;

        Runtime::emit_event(api, MintNonFungibleResourceEvent { ids })?;

        Ok(bucket)
    }

    pub(crate) fn mint_single_uuid_non_fungible<Y>(
        value: ScryptoValue,
        api: &mut Y,
    ) -> Result<(Bucket, NonFungibleLocalId), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::MUTABLE,
        )?;

        let mut resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let nf_store_id = resource_manager.non_fungible_table;
        let id_type = resource_manager.id_type;

        if id_type != NonFungibleIdType::UUID {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::InvalidNonFungibleIdType,
                ),
            ));
        }

        resource_manager.total_supply += 1;
        api.sys_write_substate_typed(resman_handle, &resource_manager)?;

        // TODO: Is this enough bits to prevent hash collisions?
        // TODO: Possibly use an always incrementing timestamp
        let id = NonFungibleLocalId::uuid(Runtime::generate_uuid(api)?).unwrap();
        let ids = btreeset!(id.clone());
        let non_fungibles = btreemap!(id.clone() => value);

        create_non_fungibles(
            resource_address,
            id_type,
            nf_store_id.as_node_id().clone(),
            non_fungibles,
            false,
            api,
        )?;

        api.sys_drop_lock(resman_handle)?;

        let bucket = ResourceManager(resource_address).new_non_fungible_bucket(ids.clone(), api)?;

        Runtime::emit_event(api, MintNonFungibleResourceEvent { ids })?;

        Ok((bucket, id))
    }

    pub(crate) fn mint_uuid_non_fungible<Y>(
        entries: Vec<(ScryptoValue,)>,
        api: &mut Y,
    ) -> Result<Bucket, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::MUTABLE,
        )?;

        let mut resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let nf_store_id = resource_manager.non_fungible_table;
        let id_type = resource_manager.id_type;

        if id_type != NonFungibleIdType::UUID {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::InvalidNonFungibleIdType,
                ),
            ));
        }

        let amount: Decimal = entries.len().into();
        resource_manager.total_supply += amount;
        api.sys_write_substate_typed(resman_handle, &resource_manager)?;

        let mut ids = BTreeSet::new();
        let mut non_fungibles = BTreeMap::new();
        for value in entries {
            let id = NonFungibleLocalId::uuid(Runtime::generate_uuid(api)?).unwrap();
            ids.insert(id.clone());
            non_fungibles.insert(id, value.0);
        }
        create_non_fungibles(
            resource_address,
            id_type,
            nf_store_id.as_node_id().clone(),
            non_fungibles,
            false,
            api,
        )?;

        api.sys_drop_lock(resman_handle)?;

        let bucket = ResourceManager(resource_address).new_non_fungible_bucket(ids.clone(), api)?;

        Runtime::emit_event(api, MintNonFungibleResourceEvent { ids })?;

        Ok(bucket)
    }

    pub(crate) fn update_non_fungible_data<Y>(
        id: NonFungibleLocalId,
        field_name: String,
        data: ScryptoValue,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::MUTABLE,
        )?;

        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let non_fungible_type_index = resource_manager.non_fungible_type_index;
        let non_fungible_table_id = resource_manager.non_fungible_table;
        let mutable_fields = resource_manager.mutable_fields.clone();

        let kv_schema = api.get_key_value_store_info(non_fungible_table_id.as_node_id())?;
        let schema_path = SchemaPath(vec![SchemaSubPath::Field(field_name.clone())]);
        let sbor_path = schema_path.to_sbor_path(&kv_schema.schema, non_fungible_type_index);
        let sbor_path = if let Some((sbor_path, ..)) = sbor_path {
            sbor_path
        } else {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::InvalidField(field_name),
                ),
            ));
        };

        if !mutable_fields.contains(&field_name) {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::FieldNotMutable(field_name),
                ),
            ));
        }

        let non_fungible_handle = api.lock_key_value_store_entry(
            non_fungible_table_id.as_node_id(),
            &id.to_key(),
            LockFlags::MUTABLE,
        )?;

        let mut non_fungible_entry: Option<ScryptoValue> =
            api.sys_read_substate_typed(non_fungible_handle)?;

        if let Some(ref mut non_fungible) = non_fungible_entry {
            let value = sbor_path.get_from_value_mut(non_fungible).unwrap();
            *value = data;

            api.sys_write_substate_typed(non_fungible_handle, &non_fungible_entry)?;
        } else {
            let non_fungible_global_id = NonFungibleGlobalId::new(resource_address, id);
            return Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::NonFungibleNotFound(Box::new(
                        non_fungible_global_id,
                    )),
                ),
            ));
        }

        api.sys_drop_lock(non_fungible_handle)?;

        Ok(())
    }

    pub(crate) fn non_fungible_exists<Y>(
        id: NonFungibleLocalId,
        api: &mut Y,
    ) -> Result<bool, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::read_only(),
        )?;

        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let non_fungible_table_id = resource_manager.non_fungible_table;

        let non_fungible_handle = api.lock_key_value_store_entry(
            non_fungible_table_id.as_node_id(),
            &id.to_key(),
            LockFlags::read_only(),
        )?;
        let non_fungible: Option<ScryptoValue> =
            api.sys_read_substate_typed(non_fungible_handle)?;
        let exists = matches!(non_fungible, Option::Some(..));

        Ok(exists)
    }

    pub(crate) fn get_non_fungible<Y>(
        id: NonFungibleLocalId,
        api: &mut Y,
    ) -> Result<ScryptoValue, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::read_only(),
        )?;

        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let non_fungible_table_id = resource_manager.non_fungible_table;

        let non_fungible_global_id = NonFungibleGlobalId::new(resource_address, id.clone());

        let non_fungible_handle = api.lock_key_value_store_entry(
            non_fungible_table_id.as_node_id(),
            &id.to_key(),
            LockFlags::read_only(),
        )?;
        let wrapper: Option<ScryptoValue> = api.sys_read_substate_typed(non_fungible_handle)?;
        if let Some(non_fungible) = wrapper {
            Ok(non_fungible)
        } else {
            Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::NonFungibleNotFound(Box::new(
                        non_fungible_global_id,
                    )),
                ),
            ))
        }
    }

    pub(crate) fn create_empty_bucket<Y>(api: &mut Y) -> Result<Bucket, RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        Self::create_bucket(BTreeSet::new(), api)
    }

    pub(crate) fn create_bucket<Y>(
        ids: BTreeSet<NonFungibleLocalId>,
        api: &mut Y,
    ) -> Result<Bucket, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::read_only(),
        )?;

        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let id_type = resource_manager.id_type;
        let bucket_id = api.new_object(
            NON_FUNGIBLE_BUCKET_BLUEPRINT,
            vec![
                scrypto_encode(&BucketInfoSubstate {
                    resource_type: ResourceType::NonFungible { id_type },
                })
                .unwrap(),
                scrypto_encode(&LiquidNonFungibleResource::new(ids)).unwrap(),
                scrypto_encode(&LockedNonFungibleResource::default()).unwrap(),
            ],
        )?;

        Ok(Bucket(Own(bucket_id)))
    }

    pub(crate) fn burn<Y>(bucket: Bucket, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::MUTABLE,
        )?;

        // Drop the bucket
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());
        let other_bucket =
            drop_non_fungible_bucket_of_address(resource_address, bucket.0.as_node_id(), api)?;

        // Construct the event and only emit it once all of the operations are done.
        Runtime::emit_event(
            api,
            BurnNonFungibleResourceEvent {
                ids: other_bucket.liquid.ids().clone(),
            },
        )?;

        // Update total supply
        // TODO: there might be better for maintaining total supply, especially for non-fungibles
        let mut resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        resource_manager.total_supply -= other_bucket.liquid.amount();

        for id in other_bucket.liquid.into_ids() {
            let non_fungible_handle = api.lock_key_value_store_entry(
                resource_manager.non_fungible_table.as_node_id(),
                &id.to_key(),
                LockFlags::MUTABLE,
            )?;

            api.sys_write_substate_typed(non_fungible_handle, None::<ScryptoValue>)?;
            api.sys_drop_lock(non_fungible_handle)?;
        }

        api.sys_write_substate_typed(resman_handle, &resource_manager)?;
        api.sys_drop_lock(resman_handle)?;

        Ok(())
    }

    pub(crate) fn drop_empty_bucket<Y>(bucket: Bucket, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let resource_address = ResourceAddress::new_or_panic(api.get_global_address()?.into());
        let other_bucket =
            drop_non_fungible_bucket_of_address(resource_address, bucket.0.as_node_id(), api)?;

        if other_bucket.liquid.amount().is_zero() {
            Ok(())
        } else {
            Err(RuntimeError::ApplicationError(
                ApplicationError::NonFungibleResourceManagerError(
                    NonFungibleResourceManagerError::DropNonEmptyBucket,
                ),
            ))
        }
    }

    pub(crate) fn create_vault<Y>(api: &mut Y) -> Result<Own, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::MUTABLE,
        )?;

        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let id_type = resource_manager.id_type;
        let info = NonFungibleVaultIdTypeSubstate { id_type };

        let ids = Own(api.new_index()?);
        let vault = LiquidNonFungibleVault {
            amount: Decimal::zero(),
            ids,
        };
        let vault_id = api.new_object(
            NON_FUNGIBLE_VAULT_BLUEPRINT,
            vec![
                scrypto_encode(&info).unwrap(),
                scrypto_encode(&vault).unwrap(),
                scrypto_encode(&LockedNonFungibleResource::default()).unwrap(),
            ],
        )?;

        Runtime::emit_event(api, VaultCreationEvent { vault_id })?;

        Ok(Own(vault_id))
    }

    pub(crate) fn get_resource_type<Y>(api: &mut Y) -> Result<ResourceType, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::read_only(),
        )?;

        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let resource_type = ResourceType::NonFungible {
            id_type: resource_manager.id_type,
        };

        Ok(resource_type)
    }

    pub(crate) fn get_total_supply<Y>(api: &mut Y) -> Result<Decimal, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let resman_handle = api.lock_field(
            ResourceManagerOffset::ResourceManager.into(),
            LockFlags::read_only(),
        )?;
        let resource_manager: NonFungibleResourceManagerSubstate =
            api.sys_read_substate_typed(resman_handle)?;
        let total_supply = resource_manager.total_supply;
        Ok(total_supply)
    }
}