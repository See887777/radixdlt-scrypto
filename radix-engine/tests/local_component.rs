use radix_engine::engine::{KernelError, ModuleError, RuntimeError};
use radix_engine::types::*;
use radix_engine_interface::data::*;
use radix_engine_interface::model::FromPublicKey;
use radix_engine_interface::node::NetworkDefinition;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

#[test]
fn local_component_should_return_correct_info() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_component");

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(
            package_address,
            "Secret",
            "check_info_of_local_component",
            args!(package_address, "Secret".to_string()),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn local_component_should_be_callable_read_only() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_component");

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(package_address, "Secret", "read_local_component", args!())
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn local_component_should_be_callable_with_write() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_component");

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(package_address, "Secret", "write_local_component", args!())
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn local_component_with_access_rules_should_not_be_callable() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_component");
    let (public_key, _, account) = test_runner.new_allocated_account();
    let auth_resource_address = test_runner.create_non_fungible_resource(account);
    let auth_id = NonFungibleId::U32(1);
    let auth_address = NonFungibleAddress::new(auth_resource_address, auth_id);

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(
            package_address,
            "Secret",
            "try_to_read_local_component_with_auth",
            args!(auth_address),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleAddress::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(e, RuntimeError::ModuleError(ModuleError::AuthError { .. }))
    });
}

#[test]
fn local_component_with_access_rules_should_be_callable() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_component");
    let (public_key, _, account) = test_runner.new_allocated_account();
    let auth_resource_address = test_runner.create_non_fungible_resource(account);
    let auth_id = NonFungibleId::U32(1);
    let auth_address = NonFungibleAddress::new(auth_resource_address, auth_id.clone());

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_method(
            account,
            "create_proof_by_ids",
            args!(BTreeSet::from([auth_id]), auth_resource_address),
        )
        .call_function(
            package_address,
            "Secret",
            "try_to_read_local_component_with_auth",
            args!(auth_address),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleAddress::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn recursion_bomb() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let (public_key, _, account) = test_runner.new_allocated_account();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_recursion");

    // Act
    // Note: currently SEGFAULT occurs if bucket with too much in it is sent. My guess the issue is a native stack overflow.
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10u32.into())
        .withdraw_from_account_by_amount(account, Decimal::from(5u32), RADIX_TOKEN)
        .take_from_worktop(RADIX_TOKEN, |builder, bucket_id| {
            builder.call_function(
                package_address,
                "LocalRecursionBomb",
                "recursion_bomb",
                args!(Bucket(bucket_id)),
            )
        })
        .call_method(
            account,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleAddress::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn recursion_bomb_to_failure() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let (public_key, _, account) = test_runner.new_allocated_account();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_recursion");

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10u32.into())
        .withdraw_from_account_by_amount(account, Decimal::from(100u32), RADIX_TOKEN)
        .take_from_worktop(RADIX_TOKEN, |builder, bucket_id| {
            builder.call_function(
                package_address,
                "LocalRecursionBomb",
                "recursion_bomb",
                args!(Bucket(bucket_id)),
            )
        })
        .call_method(
            account,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleAddress::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::KernelError(KernelError::MaxCallDepthLimitReached)
        )
    });
}

#[test]
fn recursion_bomb_2() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let (public_key, _, account) = test_runner.new_allocated_account();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_recursion");

    // Act
    // Note: currently SEGFAULT occurs if bucket with too much in it is sent. My guess the issue is a native stack overflow.
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10u32.into())
        .withdraw_from_account_by_amount(account, Decimal::from(5u32), RADIX_TOKEN)
        .take_from_worktop(RADIX_TOKEN, |builder, bucket_id| {
            builder.call_function(
                package_address,
                "LocalRecursionBomb2",
                "recursion_bomb",
                args!(Bucket(bucket_id)),
            )
        })
        .call_method(
            account,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleAddress::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn recursion_bomb_2_to_failure() {
    // Arrange
    let mut test_runner = TestRunner::new(true);
    let (public_key, _, account) = test_runner.new_allocated_account();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/local_recursion");

    // Act
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .lock_fee(FAUCET_COMPONENT, 10u32.into())
        .withdraw_from_account_by_amount(account, Decimal::from(100u32), RADIX_TOKEN)
        .take_from_worktop(RADIX_TOKEN, |builder, bucket_id| {
            builder.call_function(
                package_address,
                "LocalRecursionBomb2",
                "recursion_bomb",
                args!(Bucket(bucket_id)),
            )
        })
        .call_method(
            account,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleAddress::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::KernelError(KernelError::MaxCallDepthLimitReached)
        )
    });
}
