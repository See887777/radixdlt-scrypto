use sbor::Encode;
use scrypto::buffer::scrypto_encode;
use scrypto::rust::collections::HashMap;
use scrypto::rust::vec::Vec;

use crate::ledger::*;

/// An in-memory ledger stores all substates in host memory.
#[derive(Debug, Clone)]
pub struct InMemorySubstateStore {
    substates: HashMap<Vec<u8>, Vec<u8>>,
    child_substates: HashMap<Vec<u8>, Vec<u8>>,
    current_epoch: u64,
    nonce: u64,
}

impl InMemorySubstateStore {
    pub fn new() -> Self {
        Self {
            substates: HashMap::new(),
            child_substates: HashMap::new(),
            current_epoch: 0,
            nonce: 0,
        }
    }

    pub fn with_bootstrap() -> Self {
        let mut ledger = Self::new();
        ledger.bootstrap();
        ledger
    }
}

impl Default for InMemorySubstateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstateStore for InMemorySubstateStore {
    fn get_substate<T: Encode>(&self, address: &T) -> Option<Vec<u8>> {
        self.substates.get(&scrypto_encode(address)).cloned()
    }

    fn put_substate<T: Encode>(&mut self, address: &T, substate: &[u8]) {
        self.substates
            .insert(scrypto_encode(address), substate.to_vec());
    }

    fn get_child_substate<T: Encode>(&self, address: &T, key: &[u8]) -> Option<Vec<u8>> {
        let mut id = scrypto_encode(address);
        id.extend(key.to_vec());
        self.child_substates.get(&id).cloned()
    }

    fn put_child_substate<T: Encode>(&mut self, address: &T, key: &[u8], substate: &[u8]) {
        let mut id = scrypto_encode(address);
        id.extend(key.to_vec());
        self.child_substates.insert(id, substate.to_vec());
    }

    fn get_epoch(&self) -> u64 {
        self.current_epoch
    }

    fn set_epoch(&mut self, epoch: u64) {
        self.current_epoch = epoch;
    }

    fn get_nonce(&self) -> u64 {
        self.nonce
    }

    fn increase_nonce(&mut self) {
        self.nonce += 1;
    }
}
