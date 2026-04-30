# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.1.2] - 2026-04-30

### Changed
- Release workflow now creates a draft release first, then publishes after all assets are uploaded

## [0.1.1] - 2026-04-30

### Added
- GitHub Actions release workflow for Linux (x86_64, aarch64) and macOS (x86_64, aarch64)
- Frontend assets are now embedded into the server binary via rust-embed, producing a single self-contained binary

### Changed
- Release binary is now stripped of debug symbols, reducing size from ~4.1 MB to ~3.2 MB

### Fixed
- WebSocket connection now sends a Close frame when PTY exits, ensuring browser `onclose` fires reliably
- Broadcast sender is dropped in `handle_socket` so the channel closes properly when a session is removed

## [0.1.0] - 2026-04-08

### Added
- Browser-based terminal with WebSocket and PTY support
- Session persistence across WebSocket reconnects with scrollback replay
- Auto-removal of sessions on shell exit
