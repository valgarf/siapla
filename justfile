set windows-shell := ["nu", "-c"]

default:
    @just --list


[positional-arguments]
migrate db='sqlite:./run-data/test.sqlite' *args='':
    DATABASE_URL="{{db}}" sea-orm-cli migrate -d ./crates/siapla-migration {{args}}

generate-entity db='sqlite:./run-data/test.sqlite': (migrate "up")
    DATABASE_URL="{{db}}" sea-orm-cli generate entity \
        --with-serde both \
        --enum-extra-derives 'PartialEq','Eq','Hash' \
        --enum-extra-attributes 'WTF' \
        --expanded-format \
        -o ./crates/siapla/src/entity

[working-directory("./run-data")]
serve-backend db='sqlite:./test.sqlite':
    # pass database url to the binary via command line flag --database-url
    watchexec -d 1s -o restart -w ../crates -- cargo run -p siapla --bin siapla-serve -- --database-url "{{db}}" --bind "127.0.0.1:8880"

[working-directory("./run-data")]
serve-backend-release db='sqlite:./test.sqlite':
    watchexec -d 1s -o restart -w ../crates -- cargo run --profile release -p siapla --bin siapla-serve -- --database-url "{{db}}" --bind "127.0.0.1:8880"

[working-directory("./run-data")]
serve-backend-once db='sqlite:./test.sqlite':
    cargo run -p siapla --bin siapla-serve -- --database-url "{{db}}" --bind "127.0.0.1:8880"

[working-directory("./frontend")]
serve-frontend:
    GRAPHQL_WS="ws://localhost:8880/subscriptions" GRAPHQL_URI="http://localhost:8880/graphql" quasar dev


[working-directory("./frontend")]
build-frontend:
    # build the quasar frontend and copy the built files into the siapla crate's bundled_frontend
    quasar build
    mkdir -p ../crates/siapla/src/bundled_frontend
    rsync -rcvh dist/spa/* ../crates/siapla/src/bundled_frontend/

    # rm -rf ../crates/siapla/src/bundled_frontend
    # mkdir -p ../crates/siapla/src/bundled_frontend
    # cp -r dist/spa/* ../crates/siapla/src/bundled_frontend/


build-backend: build-frontend
    cargo build --release -p siapla

serve:
    #!/usr/bin/env bash
    just serve-backend &
    just serve-frontend &
    wait

[working-directory("./frontend")]
generate-frontend-gql:
    npm run codegen

[working-directory("./crates")]
generate-holidays-api:
    docker run --rm -u $(id -u):$(id -g)  \
    -v $PWD:/local openapitools/openapi-generator-cli generate \
    -i /local/siapla-open-holidays-api/api-definition.json \
    -g rust \
    --additional-properties packageName=siapla-open-holidays-api \
    --additional-properties packageVersion=0.1.0 \
    --additional-properties basePath=https://openholidaysapi.org \
    --type-mappings date=chrono::NaiveDate \
    --import-mappings date=chrono::NaiveDate \
    -o /local/siapla-open-holidays-api
    cargo add -p siapla-open-holidays-api chrono -F serde
    # original definition (sadly does not match real API perfectly):
    # -i https://openholidaysapi.org/swagger/v1/swagger.json \
                
[working-directory("./run-data")]
export-schema:
    cargo run -p siapla --bin siapla-export-schema


[working-directory(".")]
docker-build binary='target/release/siapla-serve' tag='siapla:latest': build-backend
    # build docker image, copying the binary via build-arg
    docker build --build-arg BINARY="{{binary}}" -t {{tag}} -f image/Dockerfile .

[working-directory(".")]
docker-run tag='siapla:latest' db_path='./run-data/test.sqlite' port='8890':
    # run container with /data mounted to local db path and port exposed
    docker stop siapla
    docker rm siapla
    docker run -d --name siapla -p {{port}}:80 -v {{db_path}}:/data/default.sqlite {{tag}}

