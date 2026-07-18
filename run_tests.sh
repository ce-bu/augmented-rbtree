#!/bin/bash

set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly core_tests=("allocator_api_tests" "allocator_tests" "augmentations_tests" "basic_tests" "drop_tests" "entry_tests" "interval_tree_tests" "iterators_tests" "rbtree_tests" "serde_tests" "topology_tests")
readonly extra_tests=("fuzz_tests" "stress_tests" "property_tests")


use_nightly=false

# UI Logging Utilities
log_info()  { echo -e "\033[32m[INFO]\033[0m $*"; }
log_warn()  { echo -e "\033[33m[WARN]\033[0m $*"; }
log_error() { echo -e "\033[31m[ERROR]\033[0m $*" >&2; }

print_usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] <command>

Commands:
  all        Run all standard tests with default features
  core       Run core test suite with selective features
  aa         Run core tests with the allocator-api feature
  miri       Run core tests using Miri isolation testing
  cov        Run branch coverage via cargo-llvm-cov
  cov2       Run branch coverage via grcov profiling
  doctest    Run core tests with the doctest feature
  opendoc    Open the generated documentation in a browser
Options:
  -n, --nightly  Force the use of the +nightly toolchain for standard tasks
  -h, --help     Display this help documentation
EOF
}


get_toolchain() {
    if [ "$use_nightly" = true ]; then
        echo "+nightly"
    else
        echo ""
    fi
}

get_test_flags() {
    local -n suite_ref=$1
    local flags=()

    flags+=("--lib")
    for t in "${suite_ref[@]}"; do
        flags+=("--test" "$t")
    done
    echo "${flags[@]}"
}

run_all() {
    local tc; tc=$(get_toolchain)
    log_info "Running all tests..."
    cargo $tc test --features serde,interval-tree
}

run_core() {
    local tc; tc=$(get_toolchain)
    log_info "Running core tests..."
    
    cargo $tc test \
        --no-default-features \
        --features alloc,serde,interval-tree \
        $(get_test_flags core_tests)

}

run_aa() {
    local tc; tc=$(get_toolchain)
    log_info "Running core tests with allocator-api..."
    
    cargo $tc test \
        --no-default-features \
        --features allocator-api,serde,interval-tree \
        $(get_test_flags core_tests)
}

run_miri() {
    log_info "Running core tests under Miri execution..."
    
    MIRIFLAGS="-Zmiri-backtrace=full" cargo +nightly miri test \
        --no-default-features \
        --features alloc,serde,interval-tree \
        $(get_test_flags core_tests)
}

run_cov() {
    log_info "Running fast llvm-cov suite..."
    
    rm -rf ./target/coverage/
    mkdir -p ./target/coverage

    if [ -d "./target/llvm-cov-target" ]; then
        find ./target/llvm-cov-target -name "*.profraw" -type f -delete
    fi

    cargo +nightly llvm-cov test \
        --no-default-features \
        --features alloc,serde,interval-tree \
        --branch \
        --no-report \
        $(get_test_flags core_tests)

    cargo +nightly llvm-cov report \
        --branch \
        --lcov \
        --output-path ./target/coverage/lcov.info \
        --ignore-filename-regex "tests/|target/"

    cargo +nightly llvm-cov report \
        --branch \
        --html \
        --output-dir ./target/coverage \
        --ignore-filename-regex "tests/|target/"
        
    log_info "Done! HTML report tracking branches available here: ./target/coverage/html/index.html"
}

run_cov2() {
    log_info "Running legacy grcov generation workflow..."
    
    rm -f *.profraw
    rm -rf ./target/grcov/

    RUSTFLAGS="-C instrument-coverage" \
    LLVM_PROFILE_FILE="rbtree-%p-%m.profraw" \
    FORCE_RUN=$(date +%s) \
        cargo +nightly test \
        --no-default-features \
        --features alloc,serde,interval-tree \
        $(get_test_flags core_tests)

    grcov . \
        --binary-path ./target/debug/ \
        --source-dir . \
        --output-type html \
        --branch \
        --ignore-not-existing \
        --ignore "tests/*" \
        --ignore "target/*" \
        --excl-line "^\s*(\/\/| \/\*|\*)" \
        --keep-only "src/*" \
        --precision 2 \
        -o ./target/grcov/html/
    
    log_info "Done! grcov HTML report generated inside ./target/grcov/html/"
}

exec_test()
{
    echo "$@"
    "$@" -- --format=pretty | awk "/^Running/"
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
        echo "Error: Cargo test suite failed!"
        exit 1
    fi
}
run_ci()
{
    cargo clippy --no-default-features --features "alloc,interval-tree,serde" -- -D warnings
    cargo clippy --no-default-features --features allocator-api,interval-tree,serde -- -D warnings

    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --no-default-features --features "nightly,interval-tree,serde"
    
    exec_test cargo test --no-default-features --features "alloc,interval-tree,serde" --release --tests 
    exec_test cargo test --no-default-features --features "allocator-api,interval-tree,serde" --release --tests 
    exec_test cargo test --no-default-features --release --tests 

    exec_test cargo +nightly test --no-default-features --features "nightly,interval-tree,serde" --release  --tests
    exec_test cargo +nightly test --no-default-features --features "alloc,nightly,interval-tree,serde" --release --tests 
    exec_test cargo test --no-default-features --release --tests

    cargo check --target thumbv7m-none-eabi --no-default-features --features "interval-tree"

    cargo  test --doc --no-default-features --features "alloc,interval-tree,serde"
    cargo +nightly test --doc --no-default-features --features "nightly,interval-tree,serde"
}

run_doctest()
{
    cargo  test --doc --no-default-features --features "alloc,interval-tree,serde"
    cargo +nightly test --doc --no-default-features --features "nightly,interval-tree,serde"
}

run_opendoc()
{
     RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --no-default-features --features "nightly,interval-tree,serde" --open
}

command_target=""

if [[ $# -eq 0 ]] || [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
    print_usage
    exit 0
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        -n|--nightly)
            use_nightly=true
            shift
            ;;
        -*)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
        *)
            command_target="$1"
            shift
            break
            ;;
    esac
done

if [[ -z "${command_target:-}" ]]; then
    log_error "Missing command name."
    print_usage
    exit 1
fi

cd "$script_dir"
case "$command_target" in
    all)  run_all ;;
    core) run_core ;;
    aa)   run_aa ;;
    miri) run_miri ;;
    cov)  run_cov ;;
    cov2) run_cov2 ;;
    ci) run_ci ;;
    doctest) run_doctest ;;
    opendoc) run_opendoc ;;
    *)
        log_error "Invalid task command name: '$command_target'"
        print_usage
        exit 1
        ;;
esac
