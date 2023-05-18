use crate::engine::scrypto_env::ScryptoEnv;
use crate::modules::ModuleHandle;
use crate::runtime::*;
use crate::*;
use radix_engine_interface::api::node_modules::royalty::{
    ComponentClaimRoyaltyInput, ComponentRoyaltyCreateInput, ComponentSetRoyaltyConfigInput,
    COMPONENT_ROYALTY_BLUEPRINT, COMPONENT_ROYALTY_CLAIM_ROYALTY_IDENT,
    COMPONENT_ROYALTY_CREATE_IDENT, COMPONENT_ROYALTY_SET_ROYALTY_CONFIG_IDENT,
};
use radix_engine_interface::api::object_api::ObjectModuleId;
use radix_engine_interface::api::ClientBlueprintApi;
use radix_engine_interface::blueprints::resource::Bucket;
use radix_engine_interface::constants::ROYALTY_MODULE_PACKAGE;
use radix_engine_interface::data::scrypto::{scrypto_decode, scrypto_encode};
use radix_engine_interface::types::RoyaltyConfig;
use scrypto::modules::Attachable;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Royalty(pub ModuleHandle);

impl Attachable for Royalty {
    const MODULE_ID: ObjectModuleId = ObjectModuleId::Royalty;

    fn new(handle: ModuleHandle) -> Self {
        Royalty(handle)
    }

    fn handle(&self) -> &ModuleHandle {
        &self.0
    }
}

impl Default for Royalty {
    fn default() -> Self {
        Royalty::new(RoyaltyConfig::default())
    }
}

impl Royalty {
    pub fn new(royalty_config: RoyaltyConfig) -> Self {
        let rtn = ScryptoEnv
            .call_function(
                ROYALTY_MODULE_PACKAGE,
                COMPONENT_ROYALTY_BLUEPRINT,
                COMPONENT_ROYALTY_CREATE_IDENT,
                scrypto_encode(&ComponentRoyaltyCreateInput { royalty_config }).unwrap(),
            )
            .unwrap();

        let royalty: Own = scrypto_decode(&rtn).unwrap();
        Self(ModuleHandle::Own(royalty))
    }

    pub fn set_config(&self, royalty_config: RoyaltyConfig) {
        self.call_ignore_rtn(
            COMPONENT_ROYALTY_SET_ROYALTY_CONFIG_IDENT,
            &ComponentSetRoyaltyConfigInput { royalty_config },
        );
    }

    pub fn claim_royalty(&self) -> Bucket {
        self.call(
            COMPONENT_ROYALTY_CLAIM_ROYALTY_IDENT,
            &ComponentClaimRoyaltyInput {},
        )
    }
}
