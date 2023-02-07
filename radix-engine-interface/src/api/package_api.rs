use crate::abi::BlueprintAbi;
use crate::api::types::*;
use crate::data::IndexedScryptoValue;
use sbor::rust::collections::BTreeMap;
use sbor::rust::vec::Vec;

pub trait ClientPackageApi<E> {
    fn call_function(
        &mut self,
        package_address: PackageAddress,
        blueprint_name: String,
        function_name: String,
        args: Vec<u8>,
    ) -> Result<IndexedScryptoValue, E>;

    fn get_code(&mut self, package_address: PackageAddress) -> Result<PackageCode, E>;

    fn get_abi(
        &mut self,
        package_address: PackageAddress,
    ) -> Result<BTreeMap<String, BlueprintAbi>, E>;
}