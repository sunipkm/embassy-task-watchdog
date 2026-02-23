#!/bin/bash
current_dir=$(pwd)
script_dir=$(dirname $(dirname $(readlink -f "$0")))

# puts us back in the current directory on failure
function cleanup {
    cd "$current_dir" || exit 1
}

# register the cleanup function to be called on the EXIT signal
trap cleanup EXIT

cd "$script_dir/embassy-task-watchdog" || exit 1

cargo +nightly docs-rs $1
