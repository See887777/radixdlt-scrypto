use crate::internal_prelude::*;

#[derive(Default, Clone, Debug, ManifestSbor, ScryptoDescribe, PartialEq, Eq)]
pub enum ManifestObjectNames {
    #[default]
    Unknown,
    Known(KnownManifestObjectNames),
}

impl From<KnownManifestObjectNames> for ManifestObjectNames {
    fn from(value: KnownManifestObjectNames) -> Self {
        Self::Known(value)
    }
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ManifestObjectNamesRef<'a> {
    #[default]
    Unknown,
    Known(&'a KnownManifestObjectNames),
}

impl HasManifestObjectNames for ManifestObjectNames {
    fn as_ref(&self) -> ManifestObjectNamesRef {
        match self {
            ManifestObjectNames::Unknown => ManifestObjectNamesRef::Unknown,
            ManifestObjectNames::Known(known) => ManifestObjectNamesRef::Known(known),
        }
    }
}

impl<'a> HasManifestObjectNames for ManifestObjectNamesRef<'a> {
    fn as_ref(&self) -> ManifestObjectNamesRef {
        *self
    }
}

pub trait HasManifestObjectNames {
    fn as_ref(&self) -> ManifestObjectNamesRef;

    fn known_bucket_name(&self, bucket: ManifestBucket) -> Option<&str> {
        match self.as_ref() {
            ManifestObjectNamesRef::Unknown => None,
            ManifestObjectNamesRef::Known(known) => known.known_bucket_name(bucket),
        }
    }

    fn bucket_name(&self, bucket: ManifestBucket) -> String {
        match self.known_bucket_name(bucket) {
            Some(name) => name.to_owned(),
            None => format!("bucket{}", bucket.0 + 1),
        }
    }

    fn known_proof_name(&self, proof: ManifestProof) -> Option<&str> {
        match self.as_ref() {
            ManifestObjectNamesRef::Unknown => None,
            ManifestObjectNamesRef::Known(known) => known.known_proof_name(proof),
        }
    }

    fn proof_name(&self, proof: ManifestProof) -> String {
        match self.known_proof_name(proof) {
            Some(name) => name.to_owned(),
            None => format!("proof{}", proof.0 + 1),
        }
    }

    fn known_address_reservation_name(
        &self,
        reservation: ManifestAddressReservation,
    ) -> Option<&str> {
        match self.as_ref() {
            ManifestObjectNamesRef::Unknown => None,
            ManifestObjectNamesRef::Known(known) => {
                known.known_address_reservation_name(reservation)
            }
        }
    }

    fn address_reservation_name(&self, reservation: ManifestAddressReservation) -> String {
        match self.known_address_reservation_name(reservation) {
            Some(name) => name.to_owned(),
            None => format!("reservation{}", reservation.0 + 1),
        }
    }

    fn known_address_name(&self, named_address: ManifestNamedAddress) -> Option<&str> {
        match self.as_ref() {
            ManifestObjectNamesRef::Unknown => None,
            ManifestObjectNamesRef::Known(known) => known.known_address_name(named_address),
        }
    }

    fn address_name(&self, named_address: ManifestNamedAddress) -> String {
        match self.known_address_name(named_address) {
            Some(name) => name.to_owned(),
            None => format!("address{}", named_address.0 + 1),
        }
    }
}

#[derive(Default, Clone, Debug, ManifestSbor, ScryptoDescribe, PartialEq, Eq)]
#[sbor(
    // This ensures that we can add new types here without
    // breaking backwards compatibility of encoded existing manifests
    as_type = "SborBackwardsCompatibleKnownManifestObjectNames",
    as_ref = "&self.into()"
)]
pub struct KnownManifestObjectNames {
    pub bucket_names: IndexMap<ManifestBucket, String>,
    pub proof_names: IndexMap<ManifestProof, String>,
    pub address_reservation_names: IndexMap<ManifestAddressReservation, String>,
    pub address_names: IndexMap<ManifestNamedAddress, String>,
}

impl HasManifestObjectNames for KnownManifestObjectNames {
    fn as_ref(&self) -> ManifestObjectNamesRef {
        ManifestObjectNamesRef::Known(self)
    }

    fn known_bucket_name(&self, bucket: ManifestBucket) -> Option<&str> {
        self.bucket_names.get(&bucket).map(|n| n.as_str())
    }

    fn known_proof_name(&self, proof: ManifestProof) -> Option<&str> {
        self.proof_names.get(&proof).map(|n| n.as_str())
    }

    fn known_address_reservation_name(
        &self,
        reservation: ManifestAddressReservation,
    ) -> Option<&str> {
        self.address_reservation_names
            .get(&reservation)
            .map(|n| n.as_str())
    }

    fn known_address_name(&self, named_address: ManifestNamedAddress) -> Option<&str> {
        self.address_names.get(&named_address).map(|n| n.as_str())
    }
}

impl KnownManifestObjectNames {
    pub fn known_bucket_name(&self, bucket: ManifestBucket) -> Option<&str> {
        self.bucket_names.get(&bucket).map(|n| n.as_str())
    }

    pub fn known_proof_name(&self, proof: ManifestProof) -> Option<&str> {
        self.proof_names.get(&proof).map(|n| n.as_str())
    }

    pub fn known_address_reservation_name(
        &self,
        reservation: ManifestAddressReservation,
    ) -> Option<&str> {
        self.address_reservation_names
            .get(&reservation)
            .map(|n| n.as_str())
    }

    pub fn known_address_name(&self, named_address: ManifestNamedAddress) -> Option<&str> {
        self.address_names.get(&named_address).map(|n| n.as_str())
    }
}

#[derive(ManifestSbor, ScryptoDescribe)]
#[sbor(transparent)]
struct SborBackwardsCompatibleKnownManifestObjectNames {
    names: BTreeMap<String, IndexMap<u32, String>>,
}

impl<'a> From<&'a KnownManifestObjectNames> for SborBackwardsCompatibleKnownManifestObjectNames {
    fn from(value: &'a KnownManifestObjectNames) -> Self {
        let mut names = BTreeMap::<String, IndexMap<u32, String>>::new();
        names.insert(
            "buckets".to_string(),
            value
                .bucket_names
                .iter()
                .map(|(b, name)| (b.0, name.to_string()))
                .collect(),
        );
        names.insert(
            "proofs".to_string(),
            value
                .proof_names
                .iter()
                .map(|(b, name)| (b.0, name.to_string()))
                .collect(),
        );
        names.insert(
            "reservations".to_string(),
            value
                .address_reservation_names
                .iter()
                .map(|(b, name)| (b.0, name.to_string()))
                .collect(),
        );
        names.insert(
            "addresses".to_string(),
            value
                .address_names
                .iter()
                .map(|(b, name)| (b.0, name.to_string()))
                .collect(),
        );
        Self { names }
    }
}

impl From<SborBackwardsCompatibleKnownManifestObjectNames> for KnownManifestObjectNames {
    fn from(mut value: SborBackwardsCompatibleKnownManifestObjectNames) -> Self {
        Self {
            bucket_names: value
                .names
                .remove("buckets")
                .map(|names| {
                    names
                        .into_iter()
                        .map(|(key, name)| (ManifestBucket(key), name))
                        .collect()
                })
                .unwrap_or_default(),
            proof_names: value
                .names
                .remove("proofs")
                .map(|names| {
                    names
                        .into_iter()
                        .map(|(key, name)| (ManifestProof(key), name))
                        .collect()
                })
                .unwrap_or_default(),
            address_reservation_names: value
                .names
                .remove("reservations")
                .map(|names| {
                    names
                        .into_iter()
                        .map(|(key, name)| (ManifestAddressReservation(key), name))
                        .collect()
                })
                .unwrap_or_default(),
            address_names: value
                .names
                .remove("addresses")
                .map(|names| {
                    names
                        .into_iter()
                        .map(|(key, name)| (ManifestNamedAddress(key), name))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}
