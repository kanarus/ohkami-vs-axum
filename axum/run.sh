#!/bin/bash

set -Cue -o pipefail

POSTGRES_URL=postgres://benchmarkdbuser:benchmarkdbpass@localhost:5432/hello_world MAX_CONNECTIONS=56 MIN_CONNECTIONS=56 cargo run --release
