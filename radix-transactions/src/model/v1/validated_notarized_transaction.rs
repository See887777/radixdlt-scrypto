use crate::internal_prelude::*;
use radix_common::constants::AuthAddresses;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidatedNotarizedTransactionV1 {
    pub prepared: PreparedNotarizedTransactionV1,
    pub encoded_instructions: Rc<Vec<u8>>,
    pub signer_keys: Vec<PublicKey>,
    pub num_of_signature_validations: usize,
}

impl HasIntentHash for ValidatedNotarizedTransactionV1 {
    fn intent_hash(&self) -> IntentHash {
        self.prepared.intent_hash()
    }
}

impl HasSignedIntentHash for ValidatedNotarizedTransactionV1 {
    fn signed_intent_hash(&self) -> SignedIntentHash {
        self.prepared.signed_intent_hash()
    }
}

impl HasNotarizedTransactionHash for ValidatedNotarizedTransactionV1 {
    fn notarized_transaction_hash(&self) -> NotarizedTransactionHash {
        self.prepared.notarized_transaction_hash()
    }
}

impl ValidatedNotarizedTransactionV1 {
    pub fn get_executable(&self) -> Executable {
        let intent = &self.prepared.signed_intent.intent;
        let header = &intent.header.inner;
        let intent_hash = intent.intent_hash();
        let summary = &self.prepared.summary;

        Executable::new(
            self.encoded_instructions.clone(),
            intent.instructions.references.clone(),
            intent.blobs.blobs_by_hash.clone(),
            ExecutionContext {
                intent_hash: TransactionIntentHash::ToCheck {
                    intent_hash: intent_hash.into_hash(),
                    expiry_epoch: header.end_epoch_exclusive,
                },
                epoch_range: Some(EpochRange {
                    start_epoch_inclusive: header.start_epoch_inclusive,
                    end_epoch_exclusive: header.end_epoch_exclusive,
                }),
                payload_size: summary.effective_length,
                num_of_signature_validations: self.num_of_signature_validations,
                auth_zone_params: AuthZoneParams {
                    initial_proofs: AuthAddresses::signer_set(&self.signer_keys),
                    virtual_resources: BTreeSet::new(),
                },
                costing_parameters: TransactionCostingParameters {
                    tip_percentage: intent.header.inner.tip_percentage,
                    free_credit_in_xrd: Decimal::ZERO,
                    abort_when_loan_repaid: false,
                },
                pre_allocated_addresses: vec![],
            },
            false,
        )
    }
}
