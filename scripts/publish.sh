#!/bin/bash
cargo publish --features dev-rp235xa,defmt-embassy-rp --target thumbv8m.main-none-eabi $1 $2