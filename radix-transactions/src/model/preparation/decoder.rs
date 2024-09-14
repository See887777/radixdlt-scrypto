use crate::internal_prelude::*;
use sbor::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValueType {
    Blob,
    Message,
    Subintent,
    ChildIntentConstraint,
    IntentSignatures,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PrepareError {
    TransactionTooLarge,
    DecodeError(DecodeError),
    EncodeError(EncodeError),
    TooManyValues {
        value_type: ValueType,
        actual: usize,
        max: usize,
    },
    LengthOverflow,
    UnexpectedDiscriminator {
        expected: u8,
        actual: u8,
    },
    Other(String),
}

impl From<DecodeError> for PrepareError {
    fn from(value: DecodeError) -> Self {
        Self::DecodeError(value)
    }
}

impl From<EncodeError> for PrepareError {
    fn from(value: EncodeError) -> Self {
        Self::EncodeError(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparationSettings {
    pub max_user_payload_length: usize,
    pub max_ledger_payload_length: usize,
    pub max_child_subintents_per_intent: usize,
    pub max_subintents_per_transaction: usize,
    pub max_blobs: usize,
}

static LATEST_PREPARATION_SETTINGS: PreparationSettings = PreparationSettings::latest();

impl PreparationSettings {
    pub const fn latest() -> Self {
        Self::cuttlefish()
    }

    pub const fn babylon() -> Self {
        let max_user_payload_length = 1 * 1024 * 1024;
        Self {
            max_user_payload_length,
            max_ledger_payload_length: max_user_payload_length + 10,
            max_child_subintents_per_intent: 0,
            max_subintents_per_transaction: 0,
            max_blobs: 64,
        }
    }

    pub const fn cuttlefish() -> Self {
        Self {
            max_child_subintents_per_intent: 128,
            max_subintents_per_transaction: 128,
            ..Self::babylon()
        }
    }

    pub fn latest_ref() -> &'static Self {
        &LATEST_PREPARATION_SETTINGS
    }

    fn check_len(
        &self,
        kind: TransactionPayloadKind,
        payload_len: usize,
    ) -> Result<(), PrepareError> {
        match kind {
            TransactionPayloadKind::CompleteUserTransaction => {
                if payload_len > self.max_user_payload_length {
                    return Err(PrepareError::TransactionTooLarge);
                }
            }
            TransactionPayloadKind::LedgerTransaction => {
                if payload_len > self.max_ledger_payload_length {
                    return Err(PrepareError::TransactionTooLarge);
                }
            }
            TransactionPayloadKind::Other => {
                // No explicit payload length checks
            }
        }
        Ok(())
    }
}

pub struct TransactionDecoder<'a> {
    decoder: ManifestDecoder<'a>,
    settings: &'a PreparationSettings,
}

impl<'a> TransactionDecoder<'a> {
    pub fn new_transaction(
        payload: &'a [u8],
        kind: TransactionPayloadKind,
        settings: &'a PreparationSettings,
    ) -> Result<Self, PrepareError> {
        settings.check_len(kind, payload.len())?;
        let mut decoder = ManifestDecoder::new(&payload, MANIFEST_SBOR_V1_MAX_DEPTH);
        decoder.read_and_check_payload_prefix(MANIFEST_SBOR_V1_PAYLOAD_PREFIX)?;
        Ok(Self { decoder, settings })
    }

    pub fn new_partial(
        payload: &'a [u8],
        settings: &'a PreparationSettings,
    ) -> Result<Self, PrepareError> {
        let mut decoder = ManifestDecoder::new(&payload, MANIFEST_SBOR_V1_MAX_DEPTH);
        decoder.read_and_check_payload_prefix(MANIFEST_SBOR_V1_PAYLOAD_PREFIX)?;
        Ok(Self { decoder, settings })
    }

    pub fn settings(&self) -> &PreparationSettings {
        &self.settings
    }

    /// Should be called before any manual call to read_X_header
    pub fn track_stack_depth_increase(&mut self) -> Result<(), PrepareError> {
        Ok(self.decoder.track_stack_depth_increase()?)
    }

    pub fn read_struct_header(&mut self, length: usize) -> Result<(), PrepareError> {
        self.read_and_check_value_kind(ValueKind::Tuple)?;
        self.read_struct_header_without_value_kind(length)
    }

    pub fn read_struct_header_without_value_kind(
        &mut self,
        length: usize,
    ) -> Result<(), PrepareError> {
        self.decoder.read_and_check_size(length)?;
        Ok(())
    }

    pub fn read_enum_header(&mut self) -> Result<(u8, usize), PrepareError> {
        self.read_and_check_value_kind(ValueKind::Enum)?;
        let discriminator = self.decoder.read_discriminator()?;
        let length = self.decoder.read_size()?;
        Ok((discriminator, length))
    }

    pub fn read_expected_enum_variant_header(
        &mut self,
        expected_discriminator: u8,
        length: usize,
    ) -> Result<(), PrepareError> {
        self.read_and_check_value_kind(ValueKind::Enum)?;
        self.read_expected_enum_variant_header_without_value_kind(expected_discriminator, length)
    }

    pub fn read_expected_enum_variant_header_without_value_kind(
        &mut self,
        expected_discriminator: u8,
        length: usize,
    ) -> Result<(), PrepareError> {
        let discriminator = self.decoder.read_discriminator()?;
        if discriminator != expected_discriminator {
            return Err(PrepareError::UnexpectedDiscriminator {
                expected: expected_discriminator,
                actual: discriminator,
            });
        }
        self.decoder.read_and_check_size(length)?;
        Ok(())
    }

    pub fn read_array_header(
        &mut self,
        element_value_kind: ManifestValueKind,
    ) -> Result<usize, PrepareError> {
        self.read_and_check_value_kind(ValueKind::Array)?;
        self.read_array_header_without_value_kind(element_value_kind)
    }

    pub fn read_array_header_without_value_kind(
        &mut self,
        element_value_kind: ManifestValueKind,
    ) -> Result<usize, PrepareError> {
        self.read_and_check_value_kind(element_value_kind)?;
        Ok(self.decoder.read_size()?)
    }

    pub fn read_and_check_value_kind(
        &mut self,
        value_kind: ManifestValueKind,
    ) -> Result<(), PrepareError> {
        self.decoder.read_and_check_value_kind(value_kind)?;
        Ok(())
    }

    /// Should be called after reading all the children following a manual read_X_header call
    pub fn track_stack_depth_decrease(&mut self) -> Result<(), PrepareError> {
        Ok(self.decoder.track_stack_depth_decrease()?)
    }

    pub fn decode<T: ManifestDecode>(&mut self) -> Result<T, PrepareError> {
        Ok(self.decoder.decode()?)
    }

    pub fn decode_deeper_body_with_value_kind<T: ManifestDecode>(
        &mut self,
        value_kind: ManifestValueKind,
    ) -> Result<T, PrepareError> {
        Ok(self
            .decoder
            .decode_deeper_body_with_value_kind(value_kind)?)
    }

    pub fn get_offset(&self) -> usize {
        self.decoder.get_offset()
    }

    pub fn get_slice_with_valid_bounds(&self, start_offset: usize, end_offset: usize) -> &[u8] {
        &self.decoder.get_input_slice()[start_offset..end_offset]
    }

    pub fn get_input_slice(&self) -> &[u8] {
        &self.decoder.get_input_slice()
    }

    pub fn check_complete(self) -> Result<(), PrepareError> {
        self.decoder.check_end()?;
        Ok(())
    }
}
