# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.1.5] - 2026-04-30

### Added
- Server connection status indicator (dot + label) in the session list toolbar, showing "Connected" or "Server offline" based on polling results
- "New Session" button is disabled automatically when the server is unreachable

## [0.1.4] - 2026-04-30

### Added
- GitHub Actions release workflow for Linux (x86_64, aarch64) and macOS (x86_64, aarch64)
- Frontend assets are now embedded into the server binary via rust-embed, producing a single self-contained binary
- CHANGELOG.md with Keep a Changelog format, wired to the release workflow

### Changed
- Release binary is now stripped of debug symbols, reducing size from ~4.1 MB to ~3.2 MB
- Release workflow creates a draft release first, then publishes after all assets are uploaded
- GitHub Actions pinned to commit SHA for supply chain security

### Fixed
- WebSocket connection now sends a Close frame when PTY exits, ensuring browser `onclose` fires reliably
- Broadcast sender is dropped in `handle_socket` so the channel closes properly when a session is removed
