#!/bin/bash
set -u
set -e
export VERSION=$1

echo "Updating to version ${VERSION}"
cargo install toml-cli
 
# RADIX_CARGO_FILES=("./scrypto-derive/Cargo.toml" "./radix-substate-store-queries/Cargo.toml" "./radix-common-derive/Cargo.toml" "./radix-engine-profiling/Cargo.toml" "./radix-substate-store-impls/Cargo.toml" "./radix-clis/Cargo.toml" "./radix-sbor-derive/Cargo.toml" "./sbor-derive/Cargo.toml" "./scrypto/Cargo.toml" "./scrypto-test/Cargo.toml" "./radix-transactions/Cargo.toml" "./radix-native-sdk/Cargo.toml" "./radix-blueprint-schema-init/Cargo.toml" "./scrypto-compiler/Cargo.toml" "./sbor-tests/Cargo.toml" "./radix-engine-interface/Cargo.toml" "./radix-rust/Cargo.toml" "./sbor-derive-common/Cargo.toml" "./radix-engine-profiling-derive/Cargo.toml" "./radix-common/Cargo.toml" "./sbor/Cargo.toml" "./scrypto-derive-tests/Cargo.toml" "./radix-engine/Cargo.toml" "./radix-transaction-scenarios/Cargo.toml" "./radix-engine-monkey-tests/Cargo.toml" "./radix-engine-tests/Cargo.toml" "./radix-substate-store-interface/Cargo.toml" "./scrypto-test/tests/blueprints/tuple-return/Cargo.toml" "./scrypto-compiler/tests/assets/scenario_1/blueprint/Cargo.toml" "./scrypto-compiler/tests/assets/scenario_1/blueprint_2/Cargo.toml" "./scrypto-compiler/tests/assets/scenario_1/blueprint_4/Cargo.toml" "./scrypto-compiler/tests/assets/scenario_1/dir/blueprint_3/Cargo.toml" )
RADIX_CARGO_FILES=$(find . -iname Cargo.toml -mindepth 2)
RADIX_CARGO_LOCK_FILES=$(find . -iname Cargo.lock -mindepth 2)
INTERNAL_PROJECT_LIST=("radix-blueprint-schema-init" "radix-common" "radix-common-derive" "radix-engine" "radix-engine-interface" "radix-engine-profiling" "radix-engine-profiling-derive" "radix-native-sdk" "radix-rust" "radix-sbor-derive" "radix-substate-store-impls" "radix-substate-store-interface" "radix-substate-store-queries" "radix-transaction-scenarios" "radix-transactions" "sbor" "sbor-derive" "sbor-derive-common" "scrypto" "scrypto-compiler" "scrypto-derive" "scrypto-test")
NUMBER_OF_PROJECTS=${#INTERNAL_PROJECT_LIST[@]}


echo "Update the package.version in all radix owned Cargo.toml files"
for toml_file in ${RADIX_CARGO_FILES[@]}; do
    FILENAME=${toml_file}
    echo "Updating ${toml_file} from $(toml get "${FILENAME}" package.version) to \"${VERSION}\""
    toml set "${FILENAME}" package.version "${VERSION}" > "${FILENAME}.new"
    mv "${FILENAME}.new" "${FILENAME}"
done

echo "Update the package.version in all radix owned Cargo.lock files"
for toml_lock_file in ${RADIX_CARGO_LOCK_FILES[@]}; do
    NUMBER_OF_PACKAGES_IN_LOCKFILE=$(toml get $toml_lock_file package | jq length)

    echo "Update the package.version of all radix owned projects in $toml_lock_file file"
    for (( i=0; i<$NUMBER_OF_PACKAGES_IN_LOCKFILE; i++ ))
    do
        value=$(toml get $toml_lock_file "package[$i].name" -r);
        if [[ "${INTERNAL_PROJECT_LIST[@]}" =~ $value && $value != "toml" ]]; then
            toml set "$toml_lock_file" "package[$i].version" "${VERSION}" > "$toml_lock_file.new"
            mv $toml_lock_file.new $toml_lock_file
        fi;
    done
done

echo "Update workspace dependencies in Cargo.toml"
for (( i=0; i<$NUMBER_OF_PROJECTS; i++ ))
do
    set +e
    value=$(toml get Cargo.toml "workspace.dependencies.${INTERNAL_PROJECT_LIST[$i]}" -r);
    ret=$?
    set -e
    if [ $ret -wq 0 ]; then
        echo "File is ${INTERNAL_PROJECT_LIST[$i]} Value is$value"
        toml set Cargo.toml "workspace.dependencies.${INTERNAL_PROJECT_LIST[$i]}.version" "${VERSION}" > Cargo.toml.new
        mv Cargo.toml.new Cargo.toml
    fi
done

for toml_file in ${RADIX_CARGO_FILES[@]}; do
    echo "Update dependencies of $toml_file"
    for (( i=0; i<$NUMBER_OF_PROJECTS; i++ ))
    do
        set +e
        value=$(toml get $toml_file "dependencies.${INTERNAL_PROJECT_LIST[$i]}" -r);
        ret=$?
        set -e
        if [ $ret -eq 0 ]; then
            echo "Setting ${INTERNAL_PROJECT_LIST[$i]} version dependency from $value to ${VERSION}"
            toml set $toml_file "dependencies.${INTERNAL_PROJECT_LIST[$i]}.version" "${VERSION}" > $toml_file.new
            mv $toml_file.new $toml_file
        fi
    done
done

toml set scrypto-test/assets/blueprints/Cargo.lock package.test_environment.version "${VERSION}" > scrypto-test/assets/blueprints/Cargo.lock.new
mv scrypto-test/assets/blueprints/Cargo.lock.new scrypto-test/assets/blueprints/Cargo.lock

toml set examples/everything/Cargo.toml_for_scrypto_builder package.version "${VERSION}" > examples/everything/Cargo.toml_for_scrypto_builder.new
mv examples/everything/Cargo.toml_for_scrypto_builder.new examples/everything/Cargo.toml_for_scrypto_builder

./update-cargo-locks.sh

echo "Done"