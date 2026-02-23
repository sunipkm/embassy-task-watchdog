#!/bin/bash
# This script builds the examples for all features/targets, to make sure they compile correctly.
current_dir=$(pwd)
script_dir=$(dirname $(dirname "$0"))

# puts us back in the current directory on failure
function cleanup {
    cd "$current_dir" || exit 1
}

# register the cleanup function to be called on the EXIT signal
trap cleanup EXIT

echo $script_dir
echo "Building examples..."
directories="$script_dir/examples/*"
echo $directories
for dir in $script_dir/examples/*; do
    if [ -d "$dir" ]; then
        echo "Building $dir..."
        cd "$dir"
        cargo build --release
        cd "$current_dir"
    fi
done
