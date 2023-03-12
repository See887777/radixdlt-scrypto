use radix_engine::errors::{InterpreterError, RuntimeError};
use radix_engine::types::*;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;
use ExpectedResult::{InvalidInput, InvalidOutput, Success};

#[test]
fn should_be_able_to_create_access_rules_with_undefined_method_name() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/package_schema");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(
            package_address,
            "SchemaComponent",
            "create_component_with_access_rules_containing_undefined_method_name",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

enum ExpectedResult {
    Success,
    InvalidInput,
    InvalidOutput,
}

fn test_arg(method_name: &str, args: Vec<u8>, expected_result: ExpectedResult) {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/package_schema");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_function(package_address, "SchemaComponent2", method_name, args)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    match expected_result {
        Success => {
            receipt.expect_commit_success();
        }
        InvalidInput => {
            receipt.expect_specific_failure(|e| {
                matches!(
                    e,
                    RuntimeError::InterpreterError(
                        InterpreterError::ScryptoInputSchemaNotMatch { .. }
                    )
                )
            });
        }
        InvalidOutput => {
            receipt.expect_specific_failure(|e| {
                matches!(
                    e,
                    RuntimeError::InterpreterError(
                        InterpreterError::ScryptoOutputSchemaNotMatch { .. }
                    )
                )
            });
        }
    }
}

#[test]
fn test_invalid_output_fails() {
    test_arg(
        "invalid_output",
        manifest_encode(&()).unwrap(),
        InvalidOutput,
    )
}

#[test]
fn test_input_arg_unit_succeeds() {
    test_arg("unit", manifest_encode(&()).unwrap(), Success)
}

#[test]
fn test_invalid_input_arg_unit_fails() {
    test_arg("unit", manifest_encode(&0u8).unwrap(), InvalidInput)
}

#[test]
fn test_input_arg_bool_succeeds() {
    test_arg("bool", manifest_encode(&true).unwrap(), Success)
}

#[test]
fn test_invalid_input_arg_bool_fails() {
    test_arg("unit", manifest_encode(&0u8).unwrap(), InvalidInput)
}

#[test]
fn test_input_arg_ivalue_succeeds() {
    test_arg("i8", manifest_encode(&0i8).unwrap(), Success);
    test_arg("i16", manifest_encode(&0i16).unwrap(), Success);
    test_arg("i32", manifest_encode(&0i32).unwrap(), Success);
    test_arg("i64", manifest_encode(&0i64).unwrap(), Success);
    test_arg("i128", manifest_encode(&0i128).unwrap(), Success);
}

#[test]
fn test_input_arg_ivalue_fails() {
    test_arg("i8", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("i16", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("i32", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("i64", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("i128", manifest_encode(&()).unwrap(), InvalidInput);
}

#[test]
fn test_input_arg_uvalue_succeeds() {
    test_arg("u8", manifest_encode(&0u8).unwrap(), Success);
    test_arg("u16", manifest_encode(&0u16).unwrap(), Success);
    test_arg("u32", manifest_encode(&0u32).unwrap(), Success);
    test_arg("u64", manifest_encode(&0u64).unwrap(), Success);
    test_arg("u128", manifest_encode(&0u128).unwrap(), Success);
}

#[test]
fn test_input_arg_uvalue_fails() {
    test_arg("u8", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("u16", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("u32", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("u64", manifest_encode(&()).unwrap(), InvalidInput);
    test_arg("u128", manifest_encode(&()).unwrap(), InvalidInput);
}

#[test]
fn test_input_arg_result_succeeds() {
    let okay: Result<(), ()> = Ok(());
    let error: Result<(), ()> = Err(());
    test_arg("result", manifest_encode(&okay).unwrap(), Success);
    test_arg("result", manifest_encode(&error).unwrap(), Success);
}

#[test]
fn test_invalid_input_arg_result_fails() {
    test_arg("result", manifest_encode(&0u8).unwrap(), InvalidInput);
}

#[test]
fn test_input_arg_tree_map_succeeds() {
    let mut tree_map = BTreeMap::new();
    tree_map.insert((), ());
    test_arg("tree_map", manifest_encode(&tree_map).unwrap(), Success);
}

#[test]
fn test_invalid_input_arg_tree_map_fails() {
    test_arg("tree_map", manifest_encode(&0u8).unwrap(), InvalidInput);
}

#[test]
fn test_input_arg_hash_set_succeeds() {
    let mut hash_set = HashSet::new();
    hash_set.insert(());
    test_arg("hash_set", manifest_encode(&hash_set).unwrap(), Success);
}

#[test]
fn test_invalid_input_arg_hash_set_fails() {
    test_arg("hash_set", manifest_encode(&0u8).unwrap(), InvalidInput);
}