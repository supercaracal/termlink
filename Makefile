.PHONY: build-frontend build-server build run dev

build-frontend:
	cd frontend && npm ci && npm run build

build-server:
	cd server && cargo build --release

build: build-frontend build-server

# フロントエンドを先にビルドして静的ファイルを生成してからサーバーを起動
run: build
	cd server && STATIC_DIR=static ./target/release/server

# 開発時: Vite dev server (proxy /ws -> localhost:3000) + cargo watch
dev:
	@echo "Run these in separate terminals:"
	@echo "  Terminal 1: cd server && cargo run"
	@echo "  Terminal 2: cd frontend && npm run dev"
