set windows-shell := ["nu", "-c"]

db := 'sqlite:./run-data/test.sqlite'

default:
    @just --list

# Call this once to initialize the repository at the beginning
[working-directory(".")]
init: install-frontend build-frontend build-backend install-e2e test
    mkdir -p run-data

[positional-arguments]
[working-directory(".")]
backup-db *args='':
    #!/usr/bin/env bash
    db_file="{{ db }}"
    if [[ $db_file == *sqlite ]]
    then
        db_file=${db_file#sqlite:}
        db_file_target=${db_file%.sqlite}_$(date +"%Y-%m-%d_%H-%M-%S").sqlite
        echo "Backing up db file '${db_file}' to ${db_file_target}"
        cp "${db_file}" "${db_file_target}"
    fi

[positional-arguments]
[working-directory(".")]
migrate *args='':
    DATABASE_URL="{{ db }}" sea-orm-cli migrate -d ./crates/siapla-migration {{ args }}

[working-directory(".")]
generate-entity: (migrate "up")
    DATABASE_URL="{{ db }}" sea-orm-cli generate entity \
        --with-serde both \
        --enum-extra-derives 'PartialEq','Eq','Hash' \
        --enum-extra-attributes 'WTF' \
        --expanded-format \
        -o ./crates/siapla/src/entity
    python ./scripts/patch_generated_entities.py

[working-directory(".")]
serve-backend:
    # pass database url to the binary via command line flag --database-url
    watchexec -d 1s -o restart -w crates -- cargo run -p siapla --bin siapla-serve -- --database-url "{{ db }}" --bind "127.0.0.1:8880" --allow-reset

[working-directory(".")]
serve-backend-release:
    watchexec -d 1s -o restart -w crates -- cargo run --profile release -p siapla --bin siapla-serve -- --database-url "{{ db }}" --bind "127.0.0.1:8880" --allow-reset

[working-directory(".")]
serve-backend-once:
    cargo run -p siapla --bin siapla-serve -- --database-url "{{ db }}" --bind "127.0.0.1:8880" --allow-reset

[working-directory("./frontend")]
serve-frontend:
    GRAPHQL_WS="ws://localhost:8880/subscriptions" GRAPHQL_URI="http://localhost:8880/graphql" quasar dev

[working-directory("./frontend")]
quasar-build:
    # build the quasar frontend
    quasar build

[working-directory("./frontend")]
build-frontend: quasar-build
    # copy the built files into the siapla crate's bundled_frontend
    quasar build
    mkdir -p ../crates/siapla/src/bundled_frontend
    rsync -rcvh dist/spa/* ../crates/siapla/src/bundled_frontend/

    # rm -rf ../crates/siapla/src/bundled_frontend
    # mkdir -p ../crates/siapla/src/bundled_frontend
    # cp -r dist/spa/* ../crates/siapla/src/bundled_frontend/

[working-directory(".")]
build-backend: build-frontend
    cargo build --release -p siapla

[working-directory("./frontend")]
install-frontend:
    npm install

serve:
    #!/usr/bin/env bash
    just serve-backend &
    just serve-frontend &
    wait

[working-directory("./frontend")]
generate-frontend-gql: export-schema
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

[working-directory("./frontend/src/gql")]
export-schema:
    cargo run -p siapla --bin siapla-export-schema

[working-directory(".")]
docker-build binary='target/release/siapla-serve' tag='siapla:latest': build-backend
    # build docker image, copying the binary via build-arg
    docker build --build-arg BINARY="{{ binary }}" -t {{ tag }} -f image/Dockerfile .

[working-directory(".")]
docker-run tag='siapla:latest' db_path='./run-data/test.sqlite' port='8890':
    # run container with /data mounted to local db path and port exposed
    docker stop siapla
    docker rm siapla
    docker run -d --name siapla -p {{ port }}:80 -v {{ db_path }}:/data/default.sqlite {{ tag }}

# ---------------------------------------------------------------------------
# AI-friendly test commands (three levels)
# ---------------------------------------------------------------------------

# Level 1: Rust unit tests (no database needed)
[working-directory(".")]
test-unit:
    cargo test -p siapla scheduling::tests -- --nocapture

# Level 2: Backend integration tests (uses a temporary SQLite database)
[working-directory(".")]
test-integration:
    cargo test -p siapla --test graphql_integration -- --nocapture

# Level 1 + 2 combined
[working-directory(".")]
test-rust:
    cargo test -- --nocapture

# Level 3: Playwright E2E tests (requires running backend + frontend)
[working-directory("./tests/e2e")]
install-e2e:
    npm install
    npx playwright install --with-deps chromium

[working-directory("./tests/e2e")]
test-e2e:
    npx playwright test

[working-directory("./tests/e2e")]
test-e2e-smoke:
    npx playwright test tests/smoke.spec.ts

[working-directory("./tests/e2e")]
test-e2e-resources:
    npx playwright test tests/resources.spec.ts

[working-directory("./tests/e2e")]
test-e2e-tasks:
    npx playwright test tests/tasks.spec.ts

[working-directory("./tests/e2e")]
test-e2e-planning:
    npx playwright test tests/planning.spec.ts

[working-directory("./tests/e2e")]
test-e2e-revisions:
    npx playwright test tests/revisions.spec.ts

[working-directory("./tests/e2e")]
test-e2e-task-history:
    npx playwright test tests/task-history.spec.ts

# Run all tests (unit + integration; E2E requires servers to be running separately)
[working-directory(".")]
test: test-rust
    @echo "Rust tests passed. For E2E tests, backup db, start backend+frontend and run: just test-e2e"
