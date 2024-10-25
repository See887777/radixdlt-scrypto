use radix_common::prelude::*;
use radix_engine_tests::common::*;
use scrypto_test::prelude::*;

#[test]
fn test_many_vectors_of_urls_metadata() {
    let mut ledger = LedgerSimulatorBuilder::new().build();
    let package_address = ledger.publish_package_simple(PackageLoader::get("metadata3"));
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package_address, "M", "f", manifest_args!())
        .build();
    let start = std::time::Instant::now();
    let receipt = ledger.execute_manifest(manifest, vec![]);
    let end = std::time::Instant::now();

    // Usage:
    // ```
    // cargo test --release --package radix-engine-tests --test system_folder -- system::metadata3::test_many_vectors_of_urls_metadata --exact --show-output
    // ```
    println!("{:?}", receipt);
    println!("{} ms", end.duration_since(start).as_millis());
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::CostingError(
                CostingError::FeeReserveError(FeeReserveError::LimitExceeded { .. }),
            ))
        )
    });
}
