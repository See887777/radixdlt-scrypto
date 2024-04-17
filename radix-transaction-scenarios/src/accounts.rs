#![allow(deprecated)]

use crate::internal_prelude::*;
use radix_transactions::prelude::*;

use crate::internal_prelude::ScenarioCore;

/// A virtual account as well as it's controlling private key.
///
/// # Deprecated
///
/// Any scenario that is run after genesis can not use virtual accounts as users could find private
/// key we're using (these are public private key anyway and we're not trying to hide them) and
/// change the account's settings or perhaps even the access rule of the account leading scenarios
/// to fail. An allocated account **MUST** be used for any scenario that runs after genesis.
#[deprecated = "Use an allocated account instead"]
pub struct VirtualAccount {
    pub key: PrivateKey,
    pub public_key: PublicKey,
    pub address: ComponentAddress,
}

impl VirtualAccount {
    pub fn for_private_key(key: PrivateKey) -> Self {
        let public_key: PublicKey = key.public_key();
        let account_address = ComponentAddress::virtual_account_from_public_key(&public_key);
        Self {
            key,
            address: account_address,
            public_key,
        }
    }

    pub fn encode(&self, context: &ScenarioCore) -> String {
        self.address
            .to_string(AddressDisplayContext::with_encoder(&context.encoder()))
    }
}

pub enum AccountIdentifier {
    One,
    Two,
    Three,
}

impl From<&VirtualAccount> for GlobalAddress {
    fn from(val: &VirtualAccount) -> Self {
        val.address.into()
    }
}

pub fn secp256k1_account_from_u64(val: u64) -> VirtualAccount {
    VirtualAccount::for_private_key(
        Secp256k1PrivateKey::from_u64(val)
            .expect("Should be valid")
            .into(),
    )
}

pub fn secp256k1_account_1() -> VirtualAccount {
    secp256k1_account_from_u64(33311)
}

pub fn secp256k1_account_2() -> VirtualAccount {
    secp256k1_account_from_u64(32144)
}

pub fn secp256k1_account_3() -> VirtualAccount {
    secp256k1_account_from_u64(53213)
}

pub fn secp256k1_account_dashboard() -> VirtualAccount {
    secp256k1_account_from_u64(53214)
}

pub fn secp256k1_account_sandbox() -> VirtualAccount {
    secp256k1_account_from_u64(53215)
}

pub fn ed25519_account_from_u64(key: u64) -> VirtualAccount {
    VirtualAccount::for_private_key(
        Ed25519PrivateKey::from_u64(key)
            .expect("Should be valid")
            .into(),
    )
}

pub fn ed25519_account_1() -> VirtualAccount {
    ed25519_account_from_u64(12451)
}

pub fn ed25519_account_2() -> VirtualAccount {
    ed25519_account_from_u64(43213)
}

pub fn ed25519_account_3() -> VirtualAccount {
    ed25519_account_from_u64(54421)
}
