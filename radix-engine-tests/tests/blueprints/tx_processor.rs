use radix_engine::{
    blueprints::transaction_processor::{
        TransactionProcessorError, MAX_TOTAL_BLOB_SIZE_PER_INVOCATION,
    },
    errors::{ApplicationError, RuntimeError},
    types::*,
    vm::NoExtension,
};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_engine_tests::include_local_wasm_str;
use scrypto_unit::*;
use transaction::prelude::*;

#[test]
fn test_blob_replacement_beyond_blob_size_limit() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package_address = publish_test_package(&mut test_runner);

    // Act
    let blob = vec![0u8; 512 * 1024];
    let blob_hash = hash(&blob);
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            package_address,
            "Test",
            "f",
            ((0..MAX_TOTAL_BLOB_SIZE_PER_INVOCATION / blob.len() + 1)
                .map(|_| ManifestBlobRef(blob_hash.0))
                .collect::<Vec<ManifestBlobRef>>(),),
        )
        .then(|mut builder| {
            builder.add_blob(blob);
            builder
        })
        .build();
    let result = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    result.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::TransactionProcessorError(
                TransactionProcessorError::TotalBlobSizeLimitExceeded
            ))
        )
    });
}

#[test]
fn test_blob_replacement_within_blob_size_limit() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package_address = publish_test_package(&mut test_runner);

    // Act
    let blob = vec![0u8; 512 * 1024];
    let blob_hash = hash(&blob);
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            package_address,
            "Test",
            "f",
            ((0..MAX_TOTAL_BLOB_SIZE_PER_INVOCATION / blob.len())
                .map(|_| ManifestBlobRef(blob_hash.0))
                .collect::<Vec<ManifestBlobRef>>(),),
        )
        .call_function(
            package_address,
            "Test",
            "f",
            ((0..MAX_TOTAL_BLOB_SIZE_PER_INVOCATION / blob.len())
                .map(|_| ManifestBlobRef(blob_hash.0))
                .collect::<Vec<ManifestBlobRef>>(),),
        )
        .then(|mut builder| {
            builder.add_blob(blob);
            builder
        })
        .build();
    let result = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    result.expect_commit_success();
}

fn publish_test_package(
    test_runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>,
) -> PackageAddress {
    let code = wat2wasm(include_local_wasm_str!("basic_package.wat"));
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(
            None,
            code,
            single_function_package_definition("Test", "f"),
            BTreeMap::new(),
            OwnerRole::None,
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    receipt.expect_commit(true).new_package_addresses()[0]
}
