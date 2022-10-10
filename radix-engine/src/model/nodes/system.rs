use crate::engine::{AuthModule, HeapRENode, SystemApi};
use crate::fee::FeeReserve;
use crate::model::{
    HardAuthRule, HardProofRule, HardResourceOrNonFungible, InvokeError, MethodAuthorization,
    SystemSubstate,
};
use crate::types::*;
use crate::wasm::*;
use scrypto::core::{SystemCreateInput, SystemFunction};

#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub enum SystemError {
    InvalidRequestData(DecodeError),
}

#[derive(Debug, Clone, TypeId, Encode, Decode, PartialEq, Eq)]
pub struct System {
    pub info: SystemSubstate,
}

impl System {
    pub fn function_auth(func: &SystemFunction) -> Vec<MethodAuthorization> {
        match func {
            SystemFunction::Create => {
                vec![MethodAuthorization::Protected(HardAuthRule::ProofRule(
                    HardProofRule::Require(HardResourceOrNonFungible::NonFungible(
                        NonFungibleAddress::new(SYSTEM_TOKEN, AuthModule::system_id()),
                    )),
                ))]
            }
        }
    }

    pub fn method_auth(method: &SystemMethod) -> Vec<MethodAuthorization> {
        match method {
            SystemMethod::SetEpoch => {
                vec![MethodAuthorization::Protected(HardAuthRule::ProofRule(
                    HardProofRule::Require(HardResourceOrNonFungible::NonFungible(
                        NonFungibleAddress::new(SYSTEM_TOKEN, AuthModule::supervisor_id()),
                    )),
                ))]
            }
            _ => vec![],
        }
    }

    pub fn static_main<'s, Y, W, I, R>(
        func: SystemFunction,
        args: ScryptoValue,
        system_api: &mut Y,
    ) -> Result<ScryptoValue, InvokeError<SystemError>>
    where
        Y: SystemApi<'s, W, I, R>,
        W: WasmEngine<I>,
        I: WasmInstance,
        R: FeeReserve,
    {
        match func {
            SystemFunction::Create => {
                let _: SystemCreateInput = scrypto_decode(&args.raw)
                    .map_err(|e| InvokeError::Error(SystemError::InvalidRequestData(e)))?;

                let node_id = system_api
                    .node_create(HeapRENode::System(System {
                        info: SystemSubstate { epoch: 0 },
                    }))
                    .map_err(InvokeError::Downstream)?;

                let global_address = system_api
                    .node_globalize(node_id)
                    .map_err(InvokeError::Downstream)?;

                let component_address: ComponentAddress = global_address.into();

                Ok(ScryptoValue::from_typed(&component_address))
            }
        }
    }

    pub fn main<'s, Y, W, I, R>(
        component_id: ComponentId,
        method: SystemMethod,
        args: ScryptoValue,
        system_api: &mut Y,
    ) -> Result<ScryptoValue, InvokeError<SystemError>>
    where
        Y: SystemApi<'s, W, I, R>,
        W: WasmEngine<I>,
        I: WasmInstance,
        R: FeeReserve,
    {
        match method {
            SystemMethod::GetCurrentEpoch => {
                let _: SystemGetCurrentEpochInput = scrypto_decode(&args.raw)
                    .map_err(|e| InvokeError::Error(SystemError::InvalidRequestData(e)))?;

                let handle = system_api
                    .lock_substate(
                        RENodeId::System(component_id),
                        SubstateOffset::System(SystemOffset::System),
                        false,
                    )
                    .map_err(InvokeError::Downstream)?;

                let substate_ref = system_api
                    .get_ref(handle)
                    .map_err(InvokeError::Downstream)?;
                let system = substate_ref.system();

                Ok(ScryptoValue::from_typed(&system.epoch))
            }
            SystemMethod::SetEpoch => {
                let SystemSetEpochInput { epoch } = scrypto_decode(&args.raw)
                    .map_err(|e| InvokeError::Error(SystemError::InvalidRequestData(e)))?;

                let handle = system_api
                    .lock_substate(
                        RENodeId::System(component_id),
                        SubstateOffset::System(SystemOffset::System),
                        true,
                    )
                    .map_err(InvokeError::Downstream)?;

                {
                    let mut substate_mut = system_api
                        .get_mut(handle)
                        .map_err(InvokeError::Downstream)?;
                    substate_mut
                        .update(|r| {
                            r.system().epoch = epoch;
                            Ok(())
                        })
                        .map_err(InvokeError::Downstream)?;
                }

                Ok(ScryptoValue::from_typed(&()))
            }
            SystemMethod::GetTransactionHash => {
                let _: SystemGetTransactionHashInput = scrypto_decode(&args.raw)
                    .map_err(|e| InvokeError::Error(SystemError::InvalidRequestData(e)))?;
                Ok(ScryptoValue::from_typed(
                    &system_api
                        .read_transaction_hash()
                        .map_err(InvokeError::Downstream)?,
                ))
            }
        }
    }
}
