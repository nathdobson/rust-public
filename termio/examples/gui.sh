#!/bin/sh
set -u
EXAMPLE=$1
stty raw -icanon -echo
cargo run -p termio --example $EXAMPLE 2> /tmp/errors
reset