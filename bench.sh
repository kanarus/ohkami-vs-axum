#!/bin/bash

set -Cue -o pipefail

function run_wrk () {
    path="$1"

    if [ $path = "" ]; then
        echo "expected argument is not passed to 'run_wrk'"
        exit 1
    fi  

    wrk \
        -H 'Accept: */*' \
        -H 'Connection: keep-alive' \
        --connections 64 \
        --duration 5s \
        --threads 4 \
        --timeout 1s \
        "http://localhost:8000$path"
}

function run_benchmark () {
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

    target_framework="$1"

    if [ $target_framework = "" ]; then
        echo "Please pass a framework name !"
        exit 1
    fi

    echo "Starting benchmark..."

    docker run -d --rm \
        -p 5432:5432 \
        -e POSTGRES_USER=benchmarkdbuser \
        -e POSTGRES_PASSWORD=benchmarkdbpass \
        -e POSTGRES_DB=hello_world \
        -v $(pwd)/postgres:/docker-entrypoint-initdb.d \
        --name postgres \
        postgres:17-bookworm

    sleep 3s

    cd ./$target_framework && (./run.sh &)

    echo "Server of framework '$target_framework' is running"

    for path in "${paths[@]}"; do
        sleep 1m
        run_wrk "$path"
    done

    echo "Finishing benchmark..."
}

function cleanup() {
    kill $(ps aux | awk '/target\/release/ {print $2}')

    docker container stop 'postgres'

    echo "Done !"
}

run_benchmark "$1"; cleanup
