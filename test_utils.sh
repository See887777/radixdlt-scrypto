#!/bin/bash

test_runner="test"
doc_test_separately=0

setup_test_runner() {
    if cargo help nextest 2>/dev/null >&2 ; then
        test_runner="nextest run"

        # workaround for lack of doctests support for nextest
        # need to keep it until issue resolved https://github.com/nextest-rs/nextest/issues/16
        doc_test_separately=1
    fi
}

# Add '-p' refix to each create in list.
# This is for cargo command.
to_cargo_crates() {
    # input must be space-separated list of crates
    local crates="$1"
    local out=
    for c in $crates
    do
        out+="-p $c "
    done
    echo "$out"
}

test_crates_features() {
    local crates=$(to_cargo_crates "$1")
    local args="${2:-}"

    cargo $test_runner $crates $args

    if [ $doc_test_separately -eq 1 ] ; then
        tmpfile=$(mktemp)
        # this is to skip error in case of bin target
        #   https://github.com/rust-lang/rust/issues/50784
        if ! cargo test $crates --doc $args 2>$tmpfile ; then
            if ! grep -q 'error: no library targets found in package' $tmpfile ; then
                cat $tmpfile
                exit 1
            fi
        fi
    fi
}

test_packages() {
    # input must be space-separated list of packages
    local packages="${1:-}"

    for p in $packages
    do
        (cd $p; scrypto test)
    done
}

test_cli() {
    # input must be space-separated list of bash scripts
    local clis="${1:-}"
    for c in $clis
    do
        (cd radix-clis; bash $c)
    done
}

test_benchmark() {
    local crates=$(to_cargo_crates "$1")
    local args="${2:-}"
    cargo bench $crates $args
}

