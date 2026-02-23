#!/bin/bash
current_dir=$(pwd)
script_dir=$(dirname $(dirname $(readlink -f "$0")))

# puts us back in the current directory on failure
function cleanup {
    cd "$current_dir" || exit 1
}

# register the cleanup function to be called on the EXIT signal
trap cleanup EXIT

# subdirs
examples="$script_dir/examples/*"
crates="$script_dir/embassy-task*"

# run cargo fmt on all subdirs
for dir in $crates $examples; do
    if [ -d "$dir" ]; then
        echo "Formatting $dir..."
        cd "$dir"
        cargo fmt
    else
        echo "Skipping $dir, not a directory"
    fi
done