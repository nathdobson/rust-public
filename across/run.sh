#!/bin/sh
stty raw -icanon -echo
cargo run "$@" 2> /tmp/errors
reset
cat /tmp/errors
