docker run --rm \
    -p 5432:5432 \
    -e POSTGRES_USER=benchmarkdbuser \
    -e POSTGRES_PASSWORD=benchmarkdbpass \
    -e POSTGRES_DB=hello_world \
    -v $(pwd)/postgres:/docker-entrypoint-initdb.d \
    postgres:17-bookworm
    # -v $(pwd)/data:/var/lib/postgresql/data \
