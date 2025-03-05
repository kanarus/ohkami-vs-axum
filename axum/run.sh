#!/bin/bash

set -Cue -o pipefail

POSTGRES_URL=postgres://benchmarkdbuser:benchmarkdbpass@localhost:5432/hello_world \
cargo run --release
