#!/bin/bash

#set -x
set -e

cd "$(dirname "$0")"
source test_utils.sh

# This should align with check.sh, build.sh, test.sh, update-cargo-locks-minimally.sh

setup_test_runner

# echo "Testing crates..."
test_crates_features \
    "sbor \
    sbor-derive-common \
    sbor-derive \
    sbor-tests \
    scrypto \
    scrypto-compiler \
    scrypto-derive \
    scrypto-derive-tests \
    radix-sbor-derive \
    radix-common \
    radix-engine-interface \
    radix-engine \
    radix-engine-tests \
    radix-transaction-scenarios \
    radix-engine-toolkit-common \
    radix-transactions"

echo "Testing scrypto packages..."
test_packages \
   "examples/hello-world \
    examples/no-std"

echo "Testing CLIs..."
(cd radix-clis; cargo test)
test_cli \
    "./tests/resim.sh \
    ./tests/scrypto.sh \
    ./tests/manifest.sh"

echo "Running benchmark..."
test_benchmark  \
    "sbor-tests \
    radix-engine-tests"

./check_stack_usage.sh

echo "Congrats! All tests passed."
