#!/bin/bash
cargo publish --features dev-rp235xa,defmt-embassy-rp,dev-stm32c031c6,defmt-embassy-stm32,defmt --target thumbv8m.main-none-eabihf $1 $2