#!/usr/bin/env bash

#######################################
#                                     #
#   ____              _     _         #
#  / ___|  ___  _ __ (_) __| | ___    #
#  \___ \ / _ \| '_ \| |/ _` |/ _ \   #
#   ___) | (_) | | | | | (_| | (_) |  #
#  |____/ \___/|_| |_|_|\__,_|\___/   #
#                                     #
#######################################


# 0. Pre-installation preparations

set -euo pipefail

IFS=$'\n\t'

echo "Welcome to Sonido installer script"

read -rp "Continue? [Y/n] " confirm

[[ -z "$confirm" || "$confirm" =~ ^[Yy]$ ]] || exit 1


# 1. Check if Rust is installed

echo "Checking if Rust is installed..."

if ! command -v rustup &> /dev/null; then
    echo "Rust is not installed. Installing Rust... (needed to compile Sonido app)"

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    export PATH="$PATH:$HOME/.cargo/bin"

    source "$HOME/.cargo/env"

    echo "Rust has been installed"
else
    echo "Rust is already installed"
fi


# 2. Check if running in the project directory

if [ ! -f "Cargo.toml" ]; then
    echo "This script must be run from the project root"

    exit 1
fi


# 3. Clean files (in case if already installed)

echo "Cleaning old project files..."

cargo clean || true

[ -f /usr/bin/sonido ] && sudo rm -vf /usr/bin/sonido || true


# 4. Compile the Rust project

echo "Compiling Sonido..."

cargo build --release


# 5. Copy compiled binary to the `/usr/bin/` directory

echo "Copying binary file to '/usr/bin/'..."

sudo cp -v \
    ./target/release/sonido \
    /usr/bin/


# 6. After installation

echo "Sonido installed successfully"


# Success!
# Enjoy your new *blazingly fast* and *highly customizable* music player
