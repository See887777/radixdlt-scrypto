use crate::blueprints::resource::Bucket;
use crate::data::scrypto::model::*;
use crate::*;
use sbor::rust::collections::BTreeSet;
use sbor::rust::fmt::Debug;

pub const NON_FUNGIBLE_BUCKET_BLUEPRINT: &str = "NonFungibleBucket";

pub const NON_FUNGIBLE_BUCKET_TAKE_NON_FUNGIBLES_IDENT: &str = "take_non_fungibles";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct BucketTakeNonFungiblesInput {
    pub ids: BTreeSet<NonFungibleLocalId>,
}

pub type BucketTakeNonFungiblesOutput = Bucket;

pub const NON_FUNGIBLE_BUCKET_GET_NON_FUNGIBLE_LOCAL_IDS_IDENT: &str = "get_non_fungible_local_ids";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct BucketGetNonFungibleLocalIdsInput {}

pub type BucketGetNonFungibleLocalIdsOutput = BTreeSet<NonFungibleLocalId>;

// Protected

pub const NON_FUNGIBLE_BUCKET_LOCK_NON_FUNGIBLES_IDENT: &str = "lock_non_fungibles";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct BucketLockNonFungiblesInput {
    pub local_ids: BTreeSet<NonFungibleLocalId>,
}

pub type BucketLockNonFungiblesOutput = ();

// Protected

pub const NON_FUNGIBLE_BUCKET_UNLOCK_NON_FUNGIBLES_IDENT: &str = "unlock_non_fungibles";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct BucketUnlockNonFungiblesInput {
    pub local_ids: BTreeSet<NonFungibleLocalId>,
}

pub type BucketUnlockNonFungiblesOutput = ();
