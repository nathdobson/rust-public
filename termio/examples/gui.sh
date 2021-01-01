#!/bin/sh
stty raw -icanon -echo
cargo run --example gui 2> /tmp/errors
reset