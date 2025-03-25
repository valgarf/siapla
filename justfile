default:
    @just --list

[positional-arguments]
migrate *args='':
    DATABASE_URL="sqlite:./run-data/test.sqlite" sea-orm-cli migrate -d ./crates/siapla-migration "$@"

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


[working-directory("./frontend")]
serve-frontend:
    GRAPHQL_URI="http://localhost:8880/graphql" quasar dev

serve:
    #!/usr/bin/env bash
    just serve-backend &
    just serve-frontend &
    wait

[working-directory("./frontend")]
generate-frontend-gql:
    npm run codegen

[working-directory("./run-data")]
export-schema:
    cargo run -p siapla --bin siapla-export-schema
