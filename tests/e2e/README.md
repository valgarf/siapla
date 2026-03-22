# SIAPLA E2E Tests

End-to-end tests for the SIAPLA planning tool using [Playwright](https://playwright.dev/).

Note: These tests are written by AI and are mainly used by AI to find bugs in AI generated changes.

## Prerequisites

- **Node.js** >= 18
- **Backend** (Rust/Axum) running at `http://localhost:8880`
- **Frontend** (Quasar dev server) running at `http://localhost:9000`

## Setup

```bash
# From this directory (tests/e2e/)
npm install

# Install Playwright browsers (Chromium only by default)
npm run install:browsers
```

## Running Tests

> **Important:** Both the backend and frontend dev servers must be running before you execute the tests.

### Start the servers (in separate terminals)

```bash
# Terminal 1 — Backend (from project root)
cargo run --bin siapla_serve

# Terminal 2 — Frontend (from frontend/)
npm run dev
```

### Run all tests

```bash
npm test
```

### Run specific test suites

```bash
npm run test:smoke       # Basic smoke tests
npm run test:resources   # Resource management tests
npm run test:tasks       # Task management tests
npm run test:planning    # Full planning scenario tests
```

### Run with UI / debugging

```bash
npm run test:headed   # Run in headed browser mode
npm run test:ui       # Open Playwright's interactive UI
npm run test:debug    # Run with Playwright Inspector for step-by-step debugging
```

## Project Structure

```
tests/e2e/
├── package.json            # Dependencies and scripts
├── tsconfig.json           # TypeScript configuration
├── playwright.config.ts    # Playwright configuration
├── README.md               # This file
├── helpers/
│   ├── graphql.ts          # Direct GraphQL API client for test data setup
│   └── cleanup.ts          # Database cleanup between tests
└── tests/
    ├── smoke.spec.ts       # Smoke tests (page loads, navigation)
    ├── resources.spec.ts   # Resource CRUD and display tests
    ├── tasks.spec.ts       # Task management and Gantt display tests
    └── planning.spec.ts    # Full planning workflow tests
```

## Architecture

### Test Strategy

Tests follow a **setup-via-API, verify-in-UI** pattern:

1. **Before each test**, the database is cleaned via GraphQL mutations (delete all tasks, then all resources).
2. **Test data is created** by calling the GraphQL API directly (not through the UI), ensuring fast and reliable setup.
3. **Assertions are made against the UI** — verifying that the Gantt chart, sidebar, and navigation reflect the expected state.

### Key Helpers

- **`helpers/graphql.ts`** — A lightweight `fetch`-based GraphQL client that talks directly to the backend at `http://localhost:8880/graphql`. Used for creating/querying/deleting test data.
- **`helpers/cleanup.ts`** — Queries all existing tasks and resources, then deletes them in the correct order (tasks first due to foreign key constraints).

### Configuration

| Setting              | Value                          |
|----------------------|--------------------------------|
| Base URL             | `http://localhost:9000`        |
| GraphQL endpoint     | `http://localhost:8880/graphql`|
| Navigation timeout   | 30 seconds                     |
| Assertion timeout    | 10 seconds                     |
| Action timeout       | 10 seconds                     |
| Test timeout         | 60 seconds                     |
| Retries              | 1                              |
| Workers              | 1 (serial execution)           |
| Reporter             | `list` (verbose)               |

## Troubleshooting

### Tests fail with connection errors

Make sure both servers are running:
- Backend at `http://localhost:8880` (check with `curl http://localhost:8880/graphql -X POST -H "Content-Type: application/json" -d '{"query":"{ tasks { dbId } }"}'`)
- Frontend at `http://localhost:9000` (open in browser)

### Tests are flaky

- The test suite runs with `workers: 1` to avoid race conditions on the shared database.
- Each test cleans the database in `beforeEach`, so tests should be independent.
- If the Gantt chart takes time to render after data changes, increase `waitForSelector` timeouts or add explicit waits for GraphQL subscription updates.

### Browser not installed

Run `npm run install:browsers` to download the required Chromium binary.
