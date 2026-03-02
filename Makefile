DIST_DIR := dist
DOCKER_IMAGE := bosua-rust
DOCKER_TAG := latest

.PHONY: help release universal linux linux-arm64 windows clean docker-linux docker-linux-arm64 quality-check deploy
.DEFAULT_GOAL := release

help:
	@echo "🦀 Bosua Rust Build & Deploy Commands"
	@echo ""
	@echo "💻 Build Commands:"
	@echo "  release          - Build macOS release binary (current architecture)"
	@echo "  universal        - Build universal macOS binary (Intel + ARM64)"
	@echo "  linux            - Build Linux x86_64 binary"
	@echo "  linux-arm64      - Build Linux ARM64 binary"
	@echo "  windows          - Build Windows x86_64 binary"
	@echo "  clean            - Clean build artifacts"
	@echo ""
	@echo "🐳 Docker Commands:"
	@echo "  docker-linux       - Build Linux amd64 Docker image"
	@echo "  docker-linux-arm64 - Build Linux arm64 Docker image"
	@echo ""
	@echo "🧹 Quality Checks:"
	@echo "  quality-check    - Run clippy, fmt check, and tests"
	@echo ""
	@echo "🚀 Deployment Commands:"
	@echo "  deploy           - Build all targets and prepare for deployment"

release:
	@echo "🏭️  Building macOS release binary..."
	cargo build --release -p bosua-macos
	@mkdir -p $(DIST_DIR)
	cp target/release/bosua-macos $(DIST_DIR)/bosua
	du -sh $(DIST_DIR)/bosua

universal:
	@echo "🏭️  Building macOS universal binary..."
	cargo build --release -p bosua-macos --target x86_64-apple-darwin
	cargo build --release -p bosua-macos --target aarch64-apple-darwin
	@mkdir -p $(DIST_DIR)
	lipo -create -output $(DIST_DIR)/bosua-macos-universal \
		target/x86_64-apple-darwin/release/bosua-macos \
		target/aarch64-apple-darwin/release/bosua-macos
	du -sh $(DIST_DIR)/bosua-universal

linux:
	@echo "🏭️  Building Linux x86_64 binary..."
	cargo build --release -p bosua-linux --target x86_64-unknown-linux-gnu
	@mkdir -p $(DIST_DIR)
	cp target/x86_64-unknown-linux-gnu/release/bosua-linux $(DIST_DIR)/bosua-linux
	du -sh $(DIST_DIR)/bosua-linux

linux-arm64:
	@echo "🏭️  Building Linux ARM64 binary..."
	cargo build --release -p bosua-linux --target aarch64-unknown-linux-gnu
	@mkdir -p $(DIST_DIR)
	cp target/aarch64-unknown-linux-gnu/release/bosua-linux $(DIST_DIR)/bosua-linux-arm64
	du -sh $(DIST_DIR)/bosua-linux-arm64

windows:
	@echo "🏭️  Building Windows x86_64 binary..."
	cargo build --release -p bosua-macos --target x86_64-pc-windows-msvc
	@mkdir -p $(DIST_DIR)
	cp target/x86_64-pc-windows-msvc/release/bosua-macos.exe $(DIST_DIR)/bosua.exe
	du -sh $(DIST_DIR)/bosua.exe

docker-linux: linux
	docker build --platform linux/amd64 -f docker/Dockerfile.amd64 -t $(DOCKER_IMAGE):$(DOCKER_TAG) .

docker-linux-arm64: linux-arm64
	docker build --platform linux/arm64 -f docker/Dockerfile.arm64 -t $(DOCKER_IMAGE)-arm64:$(DOCKER_TAG) .

quality-check:
	@echo "🎨 Checking formatting..."
	cargo fmt --all -- --check
	@echo "🔍 Running clippy..."
	cargo clippy --workspace --all-targets -- -D warnings
	@echo "🧪 Running tests..."
	cargo test --workspace
	@echo "✅ All quality checks passed!"

clean:
	rm -rf $(DIST_DIR)

deploy: release linux linux-arm64 windows
	@echo "✅ All targets built. Artifacts in $(DIST_DIR)/"
	@ls -lh $(DIST_DIR)/
