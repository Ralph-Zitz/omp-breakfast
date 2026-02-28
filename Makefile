.PHONY: build check fmt test test-unit test-integration test-all db-up db-down db-wait

COMPOSE_TEST = docker compose -f docker-compose.yml -f docker-compose.test.yml
TEST_DB_PORT ?= 5433

## Build & Check
build:
	cargo build

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
	TEST_DB_PORT=$(TEST_DB_PORT) cargo test --test api_tests -- --ignored; \
	EXIT_CODE=$$?; \
	$(MAKE) db-down; \
	exit $$EXIT_CODE

test-all: test-unit test-integration

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
