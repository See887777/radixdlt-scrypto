use crate::internal_prelude::*;

pub type ScryptoTypeKind<L> = TypeKind<ScryptoCustomTypeKind, L>;
pub type VersionedScryptoSchema = VersionedSchema<ScryptoCustomSchema>;
pub type ScryptoTypeData<L> = TypeData<ScryptoCustomTypeKind, L>;

/// A schema for the values that a codec can decode / views as valid
#[derive(Debug, Clone, PartialEq, Eq, ManifestSbor, ScryptoSbor)]
pub enum ScryptoCustomTypeKind {
    Reference,
    Own,
    Decimal,
    PreciseDecimal,
    NonFungibleLocalId,
}

#[derive(Debug, Clone, PartialEq, Eq, ManifestSbor, ScryptoSbor)]
pub enum ScryptoCustomTypeValidation {
    Reference(ReferenceValidation),
    Own(OwnValidation),
}

#[derive(Debug, Clone, PartialEq, Eq, ManifestSbor, ScryptoSbor)]
pub enum ReferenceValidation {
    IsGlobal,
    IsGlobalPackage,
    IsGlobalComponent,
    IsGlobalResourceManager,
    IsGlobalTyped(Option<PackageAddress>, String),
    IsInternal,
    IsInternalTyped(Option<PackageAddress>, String),
}

#[derive(Debug, Clone, PartialEq, Eq, ManifestSbor, ScryptoSbor)]
pub enum OwnValidation {
    IsBucket,
    IsProof,
    IsVault,
    IsKeyValueStore,
    IsGlobalAddressReservation,
    IsTypedObject(Option<PackageAddress>, String),
}

impl OwnValidation {
    pub fn could_match_manifest_bucket(&self) -> bool {
        match self {
            OwnValidation::IsBucket => true,
            OwnValidation::IsProof => false,
            OwnValidation::IsVault => false,
            OwnValidation::IsKeyValueStore => false,
            OwnValidation::IsGlobalAddressReservation => false,
            // Hard to validate without knowing package addresses from engine, assume fine
            OwnValidation::IsTypedObject(_, _) => true,
        }
    }

    pub fn could_match_manifest_proof(&self) -> bool {
        match self {
            OwnValidation::IsBucket => false,
            OwnValidation::IsProof => true,
            OwnValidation::IsVault => false,
            OwnValidation::IsKeyValueStore => false,
            OwnValidation::IsGlobalAddressReservation => false,
            // Hard to validate without knowing package addresses from engine, assume fine
            OwnValidation::IsTypedObject(_, _) => true,
        }
    }

    pub fn could_match_manifest_address_reservation(&self) -> bool {
        match self {
            OwnValidation::IsBucket => false,
            OwnValidation::IsProof => false,
            OwnValidation::IsVault => false,
            OwnValidation::IsKeyValueStore => false,
            OwnValidation::IsGlobalAddressReservation => true,
            OwnValidation::IsTypedObject(_, _) => false,
        }
    }
}

impl ReferenceValidation {
    pub fn could_match_manifest_address(&self) -> bool {
        match self {
            ReferenceValidation::IsGlobal => true,
            ReferenceValidation::IsGlobalPackage => true,
            ReferenceValidation::IsGlobalComponent => true,
            ReferenceValidation::IsGlobalResourceManager => true,
            ReferenceValidation::IsGlobalTyped(_, _) => true,
            ReferenceValidation::IsInternal => true,
            ReferenceValidation::IsInternalTyped(_, _) => true,
        }
    }
}

impl<L: SchemaTypeLink> CustomTypeKind<L> for ScryptoCustomTypeKind {
    type CustomTypeValidation = ScryptoCustomTypeValidation;
}

impl CustomTypeValidation for ScryptoCustomTypeValidation {}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct ScryptoCustomSchema {}

lazy_static::lazy_static! {
    static ref EMPTY_SCHEMA: Schema<ScryptoCustomSchema> = {
        Schema::empty()
    };
}

impl CustomSchema for ScryptoCustomSchema {
    type CustomTypeKind<L: SchemaTypeLink> = ScryptoCustomTypeKind;
    type CustomTypeValidation = ScryptoCustomTypeValidation;

    fn linearize_type_kind(
        type_kind: Self::CustomTypeKind<RustTypeId>,
        _type_indices: &IndexSet<TypeHash>,
    ) -> Self::CustomTypeKind<LocalTypeId> {
        match type_kind {
            ScryptoCustomTypeKind::Reference => ScryptoCustomTypeKind::Reference,
            ScryptoCustomTypeKind::Own => ScryptoCustomTypeKind::Own,
            ScryptoCustomTypeKind::Decimal => ScryptoCustomTypeKind::Decimal,
            ScryptoCustomTypeKind::PreciseDecimal => ScryptoCustomTypeKind::PreciseDecimal,
            ScryptoCustomTypeKind::NonFungibleLocalId => ScryptoCustomTypeKind::NonFungibleLocalId,
        }
    }

    fn resolve_well_known_type(
        well_known_id: WellKnownTypeId,
    ) -> Option<&'static TypeData<Self::CustomTypeKind<LocalTypeId>, LocalTypeId>> {
        resolve_scrypto_well_known_type(well_known_id)
    }

    fn validate_custom_type_kind(
        _context: &SchemaContext,
        type_kind: &Self::CustomTypeKind<LocalTypeId>,
    ) -> Result<(), SchemaValidationError> {
        match type_kind {
            ScryptoCustomTypeKind::Reference
            | ScryptoCustomTypeKind::Own
            | ScryptoCustomTypeKind::Decimal
            | ScryptoCustomTypeKind::PreciseDecimal
            | ScryptoCustomTypeKind::NonFungibleLocalId => {
                // No validations
            }
        }
        Ok(())
    }

    fn validate_type_metadata_with_custom_type_kind(
        _: &SchemaContext,
        type_kind: &Self::CustomTypeKind<LocalTypeId>,
        type_metadata: &TypeMetadata,
    ) -> Result<(), SchemaValidationError> {
        // Even though they all map to the same thing, we keep the explicit match statement so that
        // we will have to explicitly check this when we add a new `ScryptoCustomTypeKind`
        match type_kind {
            ScryptoCustomTypeKind::Reference
            | ScryptoCustomTypeKind::Own
            | ScryptoCustomTypeKind::Decimal
            | ScryptoCustomTypeKind::PreciseDecimal
            | ScryptoCustomTypeKind::NonFungibleLocalId => {
                validate_childless_metadata(type_metadata)?;
            }
        }
        Ok(())
    }

    fn validate_custom_type_validation(
        _context: &SchemaContext,
        custom_type_kind: &Self::CustomTypeKind<LocalTypeId>,
        custom_type_validation: &Self::CustomTypeValidation,
    ) -> Result<(), SchemaValidationError> {
        match custom_type_kind {
            ScryptoCustomTypeKind::Reference => {
                if let ScryptoCustomTypeValidation::Reference(_) = custom_type_validation {
                    Ok(())
                } else {
                    return Err(SchemaValidationError::TypeValidationMismatch);
                }
            }
            ScryptoCustomTypeKind::Own => {
                if let ScryptoCustomTypeValidation::Own(_) = custom_type_validation {
                    Ok(())
                } else {
                    return Err(SchemaValidationError::TypeValidationMismatch);
                }
            }
            ScryptoCustomTypeKind::Decimal
            | ScryptoCustomTypeKind::PreciseDecimal
            | ScryptoCustomTypeKind::NonFungibleLocalId => {
                // All these custom type kinds only support `SchemaTypeValidation::None`.
                // If they get to this point, they have been paired with some ScryptoCustomTypeValidation
                // - which isn't valid.
                return Err(SchemaValidationError::TypeValidationMismatch);
            }
        }
    }

    fn empty_schema() -> &'static Schema<Self> {
        &EMPTY_SCHEMA
    }
}

pub trait HasSchemaHash {
    fn generate_schema_hash(&self) -> SchemaHash;
}

impl HasSchemaHash for VersionedScryptoSchema {
    fn generate_schema_hash(&self) -> SchemaHash {
        SchemaHash::from(hash(scrypto_encode(self).unwrap()))
    }
}

pub fn replace_self_package_address(
    schema: &mut VersionedScryptoSchema,
    package_address: PackageAddress,
) {
    for type_validation in &mut schema.v1_mut().type_validations {
        match type_validation {
            TypeValidation::Custom(ScryptoCustomTypeValidation::Own(
                OwnValidation::IsTypedObject(package, _),
            ))
            | TypeValidation::Custom(ScryptoCustomTypeValidation::Reference(
                ReferenceValidation::IsGlobalTyped(package, _),
            ))
            | TypeValidation::Custom(ScryptoCustomTypeValidation::Reference(
                ReferenceValidation::IsInternalTyped(package, _),
            )) => {
                if package.is_none() {
                    *package = Some(package_address)
                }
            }
            _ => {}
        }
    }
}
