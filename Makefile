MAKEFLAGS += --warn-undefined-variables
SHELL     := /bin/bash -euo pipefail

run: build
	./server/target/release/server

build: build-frontend build-server

build-frontend:
	cd frontend && npm ci && npm run build

build-server:
	cd server && cargo build --release

.PHONY: run build build-frontend build-server
