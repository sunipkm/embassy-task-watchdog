#!/bin/bash
current_dir=$(pwd)
script_dir=$(dirname $(dirname $(readlink -f "$0")))

# puts us back in the current directory on failure
function cleanup {
    cd "$current_dir" || exit 1
}

# register the cleanup function to be called on the EXIT signal
trap cleanup EXIT

"$script_dir/scripts/fmt.sh"

examples="$script_dir/examples/*"
for dir in $examples; do
    if [ -d "$dir" ]; then
        echo "Building $dir..."
        cd "$dir"
        cargo build --release || { echo "Failed to build $dir, exiting."; exit 1; }
    fi
done

cd "$script_dir/embassy-task-watchdog" || exit 1

echo "Building docs-rs documentation..."
rustup target add thumbv8m.main-none-eabihf --toolchain nightly || { echo "Failed to add target, exiting."; exit 1; }
rustup target add thumbv7m-none-eabi --toolchain nightly || { echo "Failed to add target, exiting."; exit 1; }
rustup target add thumbv6m-none-eabi --toolchain nightly || { echo "Failed to add target, exiting."; exit 1; }
cargo install cargo-docs-rs
cargo +nightly docs-rs || { echo "Failed to compile docs, exiting."; exit 1; }

dry_run=$1

# check if there are untracked/uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo "There are uncommitted changes in the repository. Running cargo publish in dry-run mode"
    dry_run="--dry-run"
fi
echo "Publishing crate..."
cargo publish --features dev-rp235xa,defmt-embassy-rp,defmt,defmt-messages $dry_run
