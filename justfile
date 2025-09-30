set windows-shell := ["nu", "-c"]

default:
    @just --list


[positional-arguments]
migrate *args='':
    DATABASE_URL="sqlite:./run-data/test.sqlite" sea-orm-cli migrate -d ./crates/siapla-migration {{args}}

generate-entity: (migrate "up")
    DATABASE_URL="sqlite:./run-data/test.sqlite" sea-orm-cli generate entity \
        --with-serde both \
        --enum-extra-derives 'PartialEq','Eq','Hash' \
        --enum-extra-attributes 'WTF' \
        --expanded-format \
        -o ./crates/siapla/src/entity

[working-directory("./run-data")]
serve-backend:
    DATABASE_URL="sqlite:./test.sqlite" watchexec -d 1s -o restart -w ../crates cargo run -p siapla --bin siapla-serve

[working-directory("./run-data")]
serve-backend-once:
    DATABASE_URL="sqlite:./test.sqlite" cargo run -p siapla --bin siapla-serve

[working-directory("./frontend")]
serve-frontend:
    GRAPHQL_URI="http://localhost:8880/graphql" GRAPHQL_WS="ws://localhost:8880/subscriptions" quasar dev

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

