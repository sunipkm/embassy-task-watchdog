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
cargo install cargo-docs-rs
cargo +nightly docs-rs || { echo "Failed to compile docs, exiting."; exit 1; }
echo "Publishing crate..."
cargo publish --features dev-rp235xa,defmt-embassy-rp,dev-stm32c031c6,defmt-embassy-stm32,defmt --target thumbv8m.main-none-eabihf $1 $2
