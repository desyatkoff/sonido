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


# 1. Clean files (in case if already installed)

cargo clean

sudo rm -v /usr/bin/sonido


# 2. Compile the Rust project

cargo build --release


# 3. Copy compiled binary to the `/usr/bin/` directory

sudo cp -v \
    ./target/release/sonido \
    /usr/bin/


# Success!
# Enjoy your new *blazingly fast* and *highly customizable* music player
