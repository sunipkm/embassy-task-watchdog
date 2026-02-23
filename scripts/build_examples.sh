#!/bin/bash
# This script builds the examples for all features/targets, to make sure they compile correctly.
current_dir=$(pwd)
script_dir=$(dirname $(dirname $(readlink -f "$0")))

# puts us back in the current directory on failure
function cleanup {
    cd "$current_dir" || exit 1
}

# register the cleanup function to be called on the EXIT signal
trap cleanup EXIT

# build examples
echo "Building examples..."
directories="$script_dir/examples/*"
for dir in $directories; do
    if [ -d "$dir" ]; then
        echo "Building $dir..."
        cd "$dir"
        cargo build --release
    fi
done
