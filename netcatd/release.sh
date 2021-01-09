#!/bin/sh

ssh nathan@ndobson-ubuntu.local -t 'cd /host/Users/nathan/Documents/workspace/rust-private/rust-public/netcatd && bash -l -c "cargo build --release --target=x86_64-unknown-linux-musl --target-dir=/tmp/target-musl --example demo && cp /tmp/target-musl/x86_64-unknown-linux-musl/release/examples/demo ./bin/netcatd"'

ssh nathan@34.94.138.49 -t 'rm -f /home/nathan/workspace/netcatd/bin/netcatd '

scp /Users/nathan/Documents/workspace/rust-private/rust-public/netcatd/bin/netcatd nathan@34.94.138.49:/home/nathan/workspace/netcatd/bin/netcatd

ssh nathan@34.94.138.49 -t 'cd /home/nathan/workspace/netcatd; ./bin/netcatd '