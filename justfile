default:
    @just --list

[positional-arguments]
migrate *args='':
    DATABASE_URL="sqlite:./run_data/test.sqlite" sea-orm-cli migrate -d ./crates/siapla-migration "$@"

generate-entity: (migrate "up")
    DATABASE_URL="sqlite:./run_data/test.sqlite" sea-orm-cli generate entity -o ./crates/siapla/src/entity  --with-serde both

[working-directory("./run_data")]
serve:
    DATABASE_URL="sqlite:./test.sqlite" cargo run -p siapla --bin siapla-serve

[working-directory("./run_data")]
export-schema:
    cargo run -p siapla --bin siapla-export-schema
