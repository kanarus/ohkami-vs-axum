#!/bin/bash

set -Cue -o pipefail

DATABASE_URL=postgres://benchmarkdbuser:benchmarkdbpass@localhost:5432/hello_world \
cargo run --release
# MAX_CONNECTIONS=56 \
# MIN_CONNECTIONS=56 \
