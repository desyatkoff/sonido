#!/usr/bin/env bash

set -euo pipefail

IFS=$'\n\t'

cargo clean || true

[ -f /usr/bin/sonido ] && sudo rm -vf /usr/bin/sonido || true

cargo build --release

sudo cp -v \
    ./target/release/sonido \
    /usr/bin/

echo "Sonido installed successfully. Enjoy your new *blazingly fast* and *highly customizable* music player"
