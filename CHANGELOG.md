# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.1.9] - 2026-05-02

### Added
- Favicon displayed in browser tab

## [0.1.8] - 2026-05-01

### Added
- Terminal auto-focuses when a session is opened, so typing works immediately without clicking
- Closing the tab or window while a terminal session is active now shows a browser confirmation dialog, preventing accidental loss of the session via Ctrl+W (Ctrl+W is reserved by browsers and cannot be intercepted by JavaScript)

### Removed
- Document-level capture keydown listener intended to suppress browser shortcuts (e.g. Ctrl+R, Ctrl+L); browser security prevents this from working reliably

## [0.1.7] - 2026-05-01

### Added
- Sessions can be named at creation time via an input field in the toolbar
- "Copy URL" button in the terminal status bar copies the session URL to the clipboard
- URL routing via the History API: navigating to a session updates the URL to `/sessions/:id`, and browser back/forward buttons work correctly
- Direct URL access to `/sessions/:id` serves the app and attaches to the session on load
- Build instructions and usage guide in README

## [0.1.6] - 2026-04-30

### Added
- Biome linter and formatter for the frontend (TypeScript/Vite)
- `make lint`, `make lint-frontend`, `make lint-server` targets to the Makefile
- Claude Code Stop hook that runs `make lint` after every response

### Fixed
- Applied Biome auto-fixes to existing frontend code: import order, formatting, and missing `type` attributes on `<button>` elements

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
