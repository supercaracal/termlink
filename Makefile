MAKEFLAGS += --warn-undefined-variables
SHELL     := /bin/bash -euo pipefail

run: build
	./server/target/release/server

build: build-frontend build-server

build-frontend:
	cd frontend && npm ci && npm run build

build-server:
	cd server && cargo build --release

lint: lint-frontend lint-server

lint-frontend:
	cd frontend && npx biome check .

lint-server:
	mkdir -p server/static
	cd server && cargo clippy --no-deps --all-targets -- -D warnings && cargo fmt -- --check

.PHONY: run build build-frontend build-server lint lint-frontend lint-server
