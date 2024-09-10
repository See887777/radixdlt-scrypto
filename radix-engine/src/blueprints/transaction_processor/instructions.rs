use crate::blueprints::transaction_processor::{
    TransactionProcessorError, TxnProcessorObjects, TxnProcessorObjectsWithApi,
};
use crate::errors::{ApplicationError, RuntimeError};
use crate::kernel::kernel_api::{KernelNodeApi, KernelSubstateApi};
use radix_common::prelude::{scrypto_encode, BlueprintId, ManifestValue, Own, ScryptoValue};
use radix_engine_interface::api::{AttachedModuleId, SystemApi};
use radix_engine_interface::blueprints::transaction_processor::InstructionOutput;
use radix_engine_interface::prelude::{IndexedScryptoValue, Proof};
use radix_native_sdk::resource::{
    NativeBucket, NativeFungibleBucket, NativeNonFungibleBucket, NativeProof, Worktop,
};
use radix_native_sdk::runtime::LocalAuthZone;
use radix_rust::prelude::*;
use radix_transactions::data::transform;
use radix_transactions::manifest::*;
use radix_transactions::prelude::*;

pub trait TxnInstruction {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError>;
}

impl TxnInstruction for InstructionV1 {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        match self {
            InstructionV1::TakeAllFromWorktop(i) => i.execute(worktop, objects, api),
            InstructionV1::TakeFromWorktop(i) => i.execute(worktop, objects, api),
            InstructionV1::TakeNonFungiblesFromWorktop(i) => i.execute(worktop, objects, api),
            InstructionV1::ReturnToWorktop(i) => i.execute(worktop, objects, api),
            InstructionV1::AssertWorktopContainsAny(i) => i.execute(worktop, objects, api),
            InstructionV1::AssertWorktopContains(i) => i.execute(worktop, objects, api),
            InstructionV1::AssertWorktopContainsNonFungibles(i) => i.execute(worktop, objects, api),
            InstructionV1::PopFromAuthZone(i) => i.execute(worktop, objects, api),
            InstructionV1::PushToAuthZone(i) => i.execute(worktop, objects, api),
            InstructionV1::CreateProofFromAuthZoneOfAmount(i) => i.execute(worktop, objects, api),
            InstructionV1::CreateProofFromAuthZoneOfNonFungibles(i) => {
                i.execute(worktop, objects, api)
            }
            InstructionV1::CreateProofFromAuthZoneOfAll(i) => i.execute(worktop, objects, api),
            InstructionV1::CreateProofFromBucketOfAmount(i) => i.execute(worktop, objects, api),
            InstructionV1::CreateProofFromBucketOfNonFungibles(i) => {
                i.execute(worktop, objects, api)
            }
            InstructionV1::CreateProofFromBucketOfAll(i) => i.execute(worktop, objects, api),
            InstructionV1::DropAuthZoneProofs(i) => i.execute(worktop, objects, api),
            InstructionV1::DropAuthZoneRegularProofs(i) => i.execute(worktop, objects, api),
            InstructionV1::DropAuthZoneSignatureProofs(i) => i.execute(worktop, objects, api),
            InstructionV1::BurnResource(i) => i.execute(worktop, objects, api),
            InstructionV1::CloneProof(i) => i.execute(worktop, objects, api),
            InstructionV1::DropProof(i) => i.execute(worktop, objects, api),
            InstructionV1::CallFunction(i) => i.execute(worktop, objects, api),
            InstructionV1::CallMethod(i) => i.execute(worktop, objects, api),
            InstructionV1::CallRoyaltyMethod(i) => i.execute(worktop, objects, api),
            InstructionV1::CallMetadataMethod(i) => i.execute(worktop, objects, api),
            InstructionV1::CallRoleAssignmentMethod(i) => i.execute(worktop, objects, api),
            InstructionV1::CallDirectVaultMethod(i) => i.execute(worktop, objects, api),
            InstructionV1::DropNamedProofs(i) => i.execute(worktop, objects, api),
            InstructionV1::DropAllProofs(i) => i.execute(worktop, objects, api),
            InstructionV1::AllocateGlobalAddress(i) => i.execute(worktop, objects, api),
        }
    }
}

impl TxnInstruction for TakeAllFromWorktop {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = worktop.take_all(self.resource_address, api)?;
        objects.create_manifest_bucket(bucket)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for TakeFromWorktop {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = worktop.take(self.resource_address, self.amount, api)?;
        objects.create_manifest_bucket(bucket)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for TakeNonFungiblesFromWorktop {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = worktop.take_non_fungibles(
            self.resource_address,
            self.ids.into_iter().collect(),
            api,
        )?;
        objects.create_manifest_bucket(bucket)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for ReturnToWorktop {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = objects.take_bucket(&self.bucket_id)?;
        worktop.put(bucket, api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for AssertWorktopContainsAny {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        _objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        worktop.assert_contains(self.resource_address, api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for AssertWorktopContains {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        _objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        worktop.assert_contains_amount(self.resource_address, self.amount, api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for AssertWorktopContainsNonFungibles {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        _objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        worktop.assert_contains_non_fungibles(
            self.resource_address,
            self.ids.into_iter().collect(),
            api,
        )?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for PopFromAuthZone {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = LocalAuthZone::pop(api)?.ok_or(RuntimeError::ApplicationError(
            ApplicationError::TransactionProcessorError(TransactionProcessorError::AuthZoneIsEmpty),
        ))?;
        objects.create_manifest_proof(proof)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for PushToAuthZone {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = objects.take_proof(&self.proof_id)?;
        LocalAuthZone::push(proof, api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for CreateProofFromAuthZoneOfAmount {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = LocalAuthZone::create_proof_of_amount(self.amount, self.resource_address, api)?;
        objects.create_manifest_proof(proof)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for CreateProofFromAuthZoneOfNonFungibles {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = LocalAuthZone::create_proof_of_non_fungibles(
            &self.ids.into_iter().collect(),
            self.resource_address,
            api,
        )?;
        objects.create_manifest_proof(proof)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for CreateProofFromAuthZoneOfAll {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = LocalAuthZone::create_proof_of_all(self.resource_address, api)?;
        objects.create_manifest_proof(proof)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for CreateProofFromBucketOfAmount {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = objects.get_bucket(&self.bucket_id)?;
        let proof = bucket.create_proof_of_amount(self.amount, api)?;
        objects.create_manifest_proof(proof.into())?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for CreateProofFromBucketOfNonFungibles {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = objects.get_bucket(&self.bucket_id)?;
        let proof = bucket.create_proof_of_non_fungibles(self.ids.into_iter().collect(), api)?;
        objects.create_manifest_proof(proof.into())?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for CreateProofFromBucketOfAll {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = objects.get_bucket(&self.bucket_id)?;
        let proof = bucket.create_proof_of_all(api)?;
        objects.create_manifest_proof(proof)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for DropAuthZoneProofs {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        _objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        LocalAuthZone::drop_proofs(api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for DropAuthZoneRegularProofs {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        _objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        LocalAuthZone::drop_regular_proofs(api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for DropAuthZoneSignatureProofs {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        _objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        LocalAuthZone::drop_signature_proofs(api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for BurnResource {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let bucket = objects.take_bucket(&self.bucket_id)?;
        let rtn = bucket.burn(api)?;

        let result = IndexedScryptoValue::from_typed(&rtn);
        objects.handle_call_return_data(&result, &worktop, api)?;
        Ok(InstructionOutput::CallReturn(result.into()))
    }
}

impl TxnInstruction for CloneProof {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = objects.get_proof(&self.proof_id)?;
        let proof = proof.clone(api)?;
        objects.create_manifest_proof(proof)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for DropProof {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let proof = objects.take_proof(&self.proof_id)?;
        proof.drop(api)?;
        Ok(InstructionOutput::None)
    }
}

fn handle_invocation<Y: SystemApi<RuntimeError> + KernelSubstateApi<L>, L: Default>(
    api: &mut Y,
    objects: &mut TxnProcessorObjects,
    worktop: &mut Worktop,
    args: ManifestValue,
    invocation_handler: impl FnOnce(&mut Y, ScryptoValue) -> Result<Vec<u8>, RuntimeError>,
) -> Result<InstructionOutput, RuntimeError> {
    let scrypto_value = {
        let mut processor_with_api = TxnProcessorObjectsWithApi {
            worktop,
            objects,
            api,
            current_total_size_of_blobs: 0,
        };
        transform(args, &mut processor_with_api)?
    };

    let rtn = invocation_handler(api, scrypto_value)?;

    let result = IndexedScryptoValue::from_vec(rtn)
        .map_err(|error| TransactionProcessorError::InvocationOutputDecodeError(error))?;
    objects.handle_call_return_data(&result, &worktop, api)?;
    Ok(InstructionOutput::CallReturn(result.into()))
}

impl TxnInstruction for CallFunction {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let package_address = objects.resolve_package_address(self.package_address)?;
        handle_invocation(api, objects, worktop, self.args, |api, args| {
            api.call_function(
                package_address,
                &self.blueprint_name,
                &self.function_name,
                scrypto_encode(&args).map_err(TransactionProcessorError::ArgsEncodeError)?,
            )
        })
    }
}

impl TxnInstruction for CallMethod {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let address = objects.resolve_global_address(self.address)?;
        handle_invocation(api, objects, worktop, self.args, |api, args| {
            api.call_method(
                address.as_node_id(),
                &self.method_name,
                scrypto_encode(&args).map_err(TransactionProcessorError::ArgsEncodeError)?,
            )
        })
    }
}

impl TxnInstruction for CallRoyaltyMethod {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let address = objects.resolve_global_address(self.address)?;
        handle_invocation(api, objects, worktop, self.args, |api, args| {
            api.call_module_method(
                address.as_node_id(),
                AttachedModuleId::Royalty,
                &self.method_name,
                scrypto_encode(&args).map_err(TransactionProcessorError::ArgsEncodeError)?,
            )
        })
    }
}

impl TxnInstruction for CallMetadataMethod {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let address = objects.resolve_global_address(self.address)?;
        handle_invocation(api, objects, worktop, self.args, |api, args| {
            api.call_module_method(
                address.as_node_id(),
                AttachedModuleId::Metadata,
                &self.method_name,
                scrypto_encode(&args).map_err(TransactionProcessorError::ArgsEncodeError)?,
            )
        })
    }
}

impl TxnInstruction for CallRoleAssignmentMethod {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let address = objects.resolve_global_address(self.address)?;
        handle_invocation(api, objects, worktop, self.args, |api, args| {
            api.call_module_method(
                address.as_node_id(),
                AttachedModuleId::RoleAssignment,
                &self.method_name,
                scrypto_encode(&args).map_err(TransactionProcessorError::ArgsEncodeError)?,
            )
        })
    }
}

impl TxnInstruction for CallDirectVaultMethod {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        handle_invocation(api, objects, worktop, self.args, |api, args| {
            api.call_direct_access_method(
                self.address.as_node_id(),
                &self.method_name,
                scrypto_encode(&args).map_err(TransactionProcessorError::ArgsEncodeError)?,
            )
        })
    }
}

impl TxnInstruction for DropNamedProofs {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        for (_, real_id) in objects.proof_mapping.drain(..) {
            let proof = Proof(Own(real_id));
            proof.drop(api).map(|_| IndexedScryptoValue::unit())?;
        }
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for DropAllProofs {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        for (_, real_id) in objects.proof_mapping.drain(..) {
            let proof = Proof(Own(real_id));
            proof.drop(api).map(|_| IndexedScryptoValue::unit())?;
        }
        LocalAuthZone::drop_proofs(api)?;
        Ok(InstructionOutput::None)
    }
}

impl TxnInstruction for AllocateGlobalAddress {
    fn execute<Y: SystemApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<L>, L: Default>(
        self,
        _worktop: &mut Worktop,
        objects: &mut TxnProcessorObjects,
        api: &mut Y,
    ) -> Result<InstructionOutput, RuntimeError> {
        let (address_reservation, address) = api.allocate_global_address(BlueprintId::new(
            &self.package_address,
            self.blueprint_name,
        ))?;
        objects.create_manifest_address_reservation(address_reservation)?;
        objects.create_manifest_address(address)?;

        Ok(InstructionOutput::None)
    }
}
