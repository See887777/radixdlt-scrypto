use super::{EpochChangeEvent, RoundChangeEvent, ValidatorCreator};
use crate::errors::ApplicationError;
use crate::errors::RuntimeError;
use crate::kernel::kernel_api::{KernelNodeApi, KernelSubstateApi};
use crate::types::*;
use native_sdk::account::Account;
use native_sdk::modules::access_rules::AccessRules;
use native_sdk::modules::metadata::Metadata;
use native_sdk::modules::royalty::ComponentRoyalty;
use native_sdk::resource::{ResourceManager, SysBucket};
use native_sdk::runtime::Runtime;
use radix_engine_interface::api::node_modules::auth::AuthAddresses;
use radix_engine_interface::api::substate_api::LockFlags;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::blueprints::epoch_manager::*;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::rule;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct EpochManagerSubstate {
    pub address: ComponentAddress, // TODO: Does it make sense for this to be stored here?
    pub validator_owner_resource: ResourceAddress,
    pub epoch: u64,
    pub round: u64,

    // TODO: Move configuration to an immutable substate
    pub rounds_per_epoch: u64,
    pub num_unstake_epochs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, ScryptoSbor)]
pub struct Validator {
    pub key: EcdsaSecp256k1PublicKey,
    pub stake: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct ValidatorSetSubstate {
    pub validator_set: BTreeMap<ComponentAddress, Validator>,
    pub epoch: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Sbor)]
pub enum EpochManagerError {
    InvalidRoundUpdate { from: u64, to: u64 },
}

pub struct EpochManagerBlueprint;

impl EpochManagerBlueprint {
    pub(crate) fn create<Y>(
        olympia_validator_token_address: [u8; 26], // TODO: Clean this up
        component_address: [u8; 26],               // TODO: Clean this up
        validator_set: BTreeMap<EcdsaSecp256k1PublicKey, ValidatorInit>,
        initial_epoch: u64,
        rounds_per_epoch: u64,
        num_unstake_epochs: u64,
        api: &mut Y,
    ) -> Result<ComponentAddress, RuntimeError>
    where
        Y: KernelNodeApi + ClientApi<RuntimeError>,
    {
        let address = ComponentAddress::EpochManager(component_address);

        let owner_resman: ResourceManager = {
            let metadata: BTreeMap<String, String> = BTreeMap::new();
            let mut access_rules = BTreeMap::new();

            // TODO: remove mint and premint all tokens
            {
                let non_fungible_local_id =
                    NonFungibleLocalId::bytes(scrypto_encode(&EPOCH_MANAGER_PACKAGE).unwrap())
                        .unwrap();
                let global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_local_id);
                access_rules.insert(Mint, (rule!(require(global_id)), rule!(deny_all)));
            }

            access_rules.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));

            let resource_manager =
                ResourceManager::new_non_fungible_with_address::<(), Y, RuntimeError>(
                    NonFungibleIdType::UUID,
                    metadata,
                    access_rules,
                    olympia_validator_token_address,
                    api,
                )?;

            resource_manager
        };

        let mut validators = BTreeMap::new();

        for (key, validator_init) in validator_set {
            let (owner_token_bucket, local_id) =
                owner_resman.mint_non_fungible_single_uuid((), api)?;
            let global_id = NonFungibleGlobalId::new(owner_resman.0, local_id.clone());

            let validator_account = Account(validator_init.validator_account_address);
            validator_account.deposit(owner_token_bucket, api)?;

            let stake = validator_init.initial_stake.sys_amount(api)?;
            let (address, lp_bucket) = ValidatorCreator::create_with_initial_stake(
                address,
                key,
                rule!(require(global_id)),
                validator_init.initial_stake,
                true,
                api,
            )?;
            let validator = Validator { key, stake };
            validators.insert(address, validator);

            let staker_account = Account(validator_init.stake_account_address);
            staker_account.deposit(lp_bucket, api)?;
        }

        let epoch_manager_id = {
            let epoch_manager = EpochManagerSubstate {
                address,
                validator_owner_resource: owner_resman.0,
                epoch: initial_epoch,
                round: 0,
                rounds_per_epoch,
                num_unstake_epochs,
            };
            let current_validator_set = ValidatorSetSubstate {
                epoch: initial_epoch,
                validator_set: validators.clone(),
            };

            let preparing_validator_set = ValidatorSetSubstate {
                epoch: initial_epoch + 1,
                validator_set: validators.clone(),
            };

            api.new_object(
                EPOCH_MANAGER_BLUEPRINT,
                vec![
                    scrypto_encode(&epoch_manager).unwrap(),
                    scrypto_encode(&current_validator_set).unwrap(),
                    scrypto_encode(&preparing_validator_set).unwrap(),
                ],
            )?
        };

        Runtime::emit_event(
            api,
            EpochChangeEvent {
                epoch: initial_epoch,
                validators,
            },
        )?;

        let mut access_rules = AccessRulesConfig::new();
        access_rules.set_method_access_rule(
            MethodKey::new(NodeModuleId::SELF, EPOCH_MANAGER_NEXT_ROUND_IDENT),
            rule!(require(AuthAddresses::validator_role())),
        );
        access_rules.set_method_access_rule(
            MethodKey::new(NodeModuleId::SELF, EPOCH_MANAGER_GET_CURRENT_EPOCH_IDENT),
            rule!(allow_all),
        );
        access_rules.set_method_access_rule(
            MethodKey::new(NodeModuleId::SELF, EPOCH_MANAGER_CREATE_VALIDATOR_IDENT),
            rule!(allow_all),
        );
        let non_fungible_local_id =
            NonFungibleLocalId::bytes(scrypto_encode(&EPOCH_MANAGER_PACKAGE).unwrap()).unwrap();
        let non_fungible_global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_local_id);
        access_rules.set_method_access_rule(
            MethodKey::new(NodeModuleId::SELF, EPOCH_MANAGER_UPDATE_VALIDATOR_IDENT),
            rule!(require(non_fungible_global_id)),
        );
        access_rules.set_method_access_rule(
            MethodKey::new(NodeModuleId::SELF, EPOCH_MANAGER_SET_EPOCH_IDENT),
            rule!(require(AuthAddresses::system_role())), // Set epoch only used for debugging
        );

        let access_rules = AccessRules::sys_new(access_rules, api)?.0;
        let metadata = Metadata::sys_create(api)?;
        let royalty = ComponentRoyalty::sys_create(api, RoyaltyConfig::default())?;

        api.globalize_with_address(
            RENodeId::Object(epoch_manager_id),
            btreemap!(
                NodeModuleId::AccessRules => access_rules.id(),
                NodeModuleId::Metadata => metadata.id(),
                NodeModuleId::ComponentRoyalty => royalty.id(),
            ),
            address.into(),
        )?;

        Ok(address)
    }

    pub(crate) fn get_current_epoch<Y>(receiver: RENodeId, api: &mut Y) -> Result<u64, RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let handle = api.sys_lock_substate(
            receiver,
            SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
            LockFlags::read_only(),
        )?;

        let epoch_manager: &EpochManagerSubstate = api.kernel_get_substate_ref(handle)?;

        Ok(epoch_manager.epoch)
    }

    pub(crate) fn next_round<Y>(
        receiver: RENodeId,
        round: u64,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let mgr_handle = api.sys_lock_substate(receiver, offset, LockFlags::MUTABLE)?;
        let epoch_manager: &mut EpochManagerSubstate =
            api.kernel_get_substate_ref_mut(mgr_handle)?;

        if round <= epoch_manager.round {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::EpochManagerError(EpochManagerError::InvalidRoundUpdate {
                    from: epoch_manager.round,
                    to: round,
                }),
            ));
        }

        if round >= epoch_manager.rounds_per_epoch {
            let offset = SubstateOffset::EpochManager(EpochManagerOffset::PreparingValidatorSet);
            let handle = api.sys_lock_substate(receiver, offset, LockFlags::MUTABLE)?;
            let preparing_validator_set: &mut ValidatorSetSubstate =
                api.kernel_get_substate_ref_mut(handle)?;
            let prepared_epoch = preparing_validator_set.epoch;
            let next_validator_set = preparing_validator_set.validator_set.clone();
            preparing_validator_set.epoch = prepared_epoch + 1;

            let epoch_manager: &mut EpochManagerSubstate =
                api.kernel_get_substate_ref_mut(mgr_handle)?;
            epoch_manager.epoch = prepared_epoch;
            epoch_manager.round = 0;

            let handle = api.sys_lock_substate(
                receiver,
                SubstateOffset::EpochManager(EpochManagerOffset::CurrentValidatorSet),
                LockFlags::MUTABLE,
            )?;
            let validator_set: &mut ValidatorSetSubstate =
                api.kernel_get_substate_ref_mut(handle)?;
            validator_set.epoch = prepared_epoch;
            validator_set.validator_set = next_validator_set.clone();

            Runtime::emit_event(
                api,
                EpochChangeEvent {
                    epoch: prepared_epoch,
                    validators: next_validator_set,
                },
            )?;
        } else {
            epoch_manager.round = round;

            Runtime::emit_event(api, RoundChangeEvent { round })?;
        }

        Ok(())
    }

    pub(crate) fn set_epoch<Y>(
        receiver: RENodeId,
        epoch: u64,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let handle = api.sys_lock_substate(
            receiver,
            SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
            LockFlags::MUTABLE,
        )?;

        let epoch_manager: &mut EpochManagerSubstate = api.kernel_get_substate_ref_mut(handle)?;
        epoch_manager.epoch = epoch;

        Ok(())
    }

    pub(crate) fn create_validator<Y>(
        receiver: RENodeId,
        key: EcdsaSecp256k1PublicKey,
        api: &mut Y,
    ) -> Result<(ComponentAddress, Bucket), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let handle = api.sys_lock_substate(
            receiver,
            SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
            LockFlags::read_only(),
        )?;
        let epoch_manager: &EpochManagerSubstate = api.kernel_get_substate_ref(handle)?;
        let manager = epoch_manager.address;

        let owner_resman = ResourceManager(epoch_manager.validator_owner_resource);
        let (owner_token_bucket, local_id) = owner_resman.mint_non_fungible_single_uuid((), api)?;
        let global_id = NonFungibleGlobalId::new(owner_resman.0, local_id.clone());

        let validator_address =
            ValidatorCreator::create(manager, key, rule!(require(global_id)), false, api)?;

        Ok((validator_address, owner_token_bucket))
    }

    pub(crate) fn update_validator<Y>(
        receiver: RENodeId,
        validator_address: ComponentAddress,
        update: UpdateValidator,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + ClientApi<RuntimeError>,
    {
        let handle = api.sys_lock_substate(
            receiver,
            SubstateOffset::EpochManager(EpochManagerOffset::PreparingValidatorSet),
            LockFlags::MUTABLE,
        )?;
        let validator_set: &mut ValidatorSetSubstate = api.kernel_get_substate_ref_mut(handle)?;
        match update {
            UpdateValidator::Register(key, stake) => {
                validator_set
                    .validator_set
                    .insert(validator_address, Validator { key, stake });
            }
            UpdateValidator::Unregister => {
                validator_set.validator_set.remove(&validator_address);
            }
        }

        Ok(())
    }
}
