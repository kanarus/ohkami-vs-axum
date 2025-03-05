#!/bin/bash

set -Cue -o pipefail

function run_wrk () {
    path="$1"

    wrk \
        -H 'Accept: */*' \
        -H 'Connection: keep-alive' \
        --connections 512 \
        --duration 5s \
        --threads 12 \
        --timeout 1s \
        "http://localhost:8000$path"
}

function run_benchmark () {
    framework="$1"
    comment="$2"

    wd="$PWD"

    docker run -d --rm \
        -p 5432:5432 \
        -e POSTGRES_USER=benchmarkdbuser \
        -e POSTGRES_PASSWORD=benchmarkdbpass \
        -e POSTGRES_DB=hello_world \
        -v $wd/postgres:/docker-entrypoint-initdb.d \
        --name postgres \
        postgres:17-bookworm

    sleep 5s

    cd ./$framework && \
    cargo build --release && \
    (./run.sh &) && \
    sleep 2 && \
    cd $wd

    echo "framework '$framework' ($comment) is running"
    
    result=''
    paths=(
        '/json'
        '/db'
        '/queries?q='
        '/queries?q=42'
        '/queries?q=1024'
        '/fortunes'
        '/updates?q='
        '/updates?q=42'
        '/updates?q=1024'
        '/plaintext'
    )
    for path in "${paths[@]}"; do
        echo
        echo "preparing benchmark for '$path'..."

        sleep 30s

        rps=$(run_wrk "$path" | awk '/^Requests\/sec/ {print $2}')
        echo "$rps reqs/sec for '$path'"
        if [ "$result" != '' ]; then
            result="$result,"
        fi
        result="$result\"$path\": $rps"
    done
    result="{$result}"

    timestamp=$(date -u +'%Y%m%d%H%M%S')
    log_jsonc="./.log/$framework-$timestamp-$comment.jsonc"
    echo "/* $comment */" >  $log_jsonc
    echo                  >> $log_jsonc
    echo $result | jq     >> $log_jsonc
    echo
    echo "Finishing benchmark..."
}

function cleanup() {
    echo "Cleaning up..."

    kill $(ps aux | awk '/target\/release/ {print $2}')
    docker container stop 'postgres'
}

if [ $# != 2 ]; then
    echo 'usage: ./bench.sh <framework> <comment (what you changed for it)>'
    exit 1
fi
(cleanup 2>&1 | cat > /dev/null) || :
echo "Starting benchmark..."
echo "For manual cleanup, run:

    kill \$(ps aux | awk '/target\\/release/ {print \$2}') ; docker container stop 'postgres'
"
sleep 5s
run_benchmark "$1" "$2"
cleanup
echo "Done !"
