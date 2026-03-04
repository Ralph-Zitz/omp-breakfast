.PHONY: build check fmt test test-unit test-integration test-frontend test-all db-up db-down db-wait \
       frontend frontend-build frontend-dev frontend-clean audit audit-install audit-if-available

COMPOSE_TEST = docker compose -f docker-compose.yml -f docker-compose.test.yml
TEST_DB_PORT ?= 5433

## Build & Check
build: frontend-build
	cargo build

## Frontend (Leptos WASM via Trunk)
frontend-build:
	@echo "Bundling CSS..."
	cd frontend && ./bundle-css.sh
	@echo "Building frontend..."
	cd frontend && trunk build --release

frontend-dev:
	@echo "Bundling CSS..."
	cd frontend && ./bundle-css.sh
	@echo "Starting Trunk dev server (http://127.0.0.1:8081)..."
	cd frontend && trunk serve --port 8081

frontend-clean:
	@echo "Cleaning frontend build artifacts..."
	rm -rf frontend/dist

check:
	cargo check

fmt:
	cargo fmt

## Testing
test: test-unit

test-unit:
	cargo test

test-integration: db-up db-wait
	@echo "Running integration tests on port $(TEST_DB_PORT)..."
	TEST_DB_PORT=$(TEST_DB_PORT) cargo test -- --ignored 2>&1 | tee /tmp/integration-test-output.txt; \
	EXIT_CODE=$${PIPESTATUS[0]}; \
	echo ""; \
	echo "=== Test Summary ==="; \
	rg "test result|FAILED|failures" /tmp/integration-test-output.txt || true; \
	rm -f /tmp/integration-test-output.txt; \
	$(MAKE) db-down; \
	exit $$EXIT_CODE

test-frontend:
	@echo "Running frontend WASM tests (headless Chrome)..."
	cd frontend && WASM_BINDGEN_TEST_TIMEOUT=120 wasm-pack test --headless --chrome

test-all: test-unit test-integration test-frontend audit-if-available

## Database lifecycle (test)
db-up:
	@echo "Starting test database on port $(TEST_DB_PORT)..."
	$(COMPOSE_TEST) up -d postgres
	$(COMPOSE_TEST) run --rm postgres-setup

db-wait:
	@echo "Waiting for test database to accept connections..."
	@for i in $$(seq 1 30); do \
		PGPASSWORD=actix psql -h localhost -p $(TEST_DB_PORT) -U actix -d actix \
			-c "SELECT 1" >/dev/null 2>&1 && echo "Database is ready." && break; \
		if [ $$i -eq 30 ]; then echo "ERROR: Database did not become ready in time." && exit 1; fi; \
		sleep 1; \
	done

db-down:
	@echo "Stopping test database..."
	$(COMPOSE_TEST) down -v

## Security
audit-install:
	@echo "Installing cargo-audit..."
	cargo install cargo-audit

audit:
	@echo "Running security audit on dependencies..."
	cargo audit --ignore RUSTSEC-2023-0071

audit-if-available:
	@if command -v cargo-audit >/dev/null 2>&1; then \
		echo "Running security audit on dependencies..."; \
		cargo audit --ignore RUSTSEC-2023-0071; \
	else \
		echo "SKIP: cargo-audit not installed (run 'make audit-install' to enable)"; \
	fi
