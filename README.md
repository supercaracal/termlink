![Release status](https://github.com/supercaracal/termlink/actions/workflows/release.yml/badge.svg)

# Termlink

A minimal tool for using a terminal via a browser.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org/) (v18+)

## Build & Run

```sh
make run
```

This builds the frontend, compiles the server, and starts it. Open http://127.0.0.1:3000 in your browser.

To change the bind address:

```sh
make run BIND_ADDR=0.0.0.0:8080
```

## Usage

1. Enter a session name (optional) and click **+ New Session** to launch a shell.
2. Click **Attach** on an existing session to reconnect.
3. Use **← Sessions** to return to the session list, or **Copy URL** to share a direct link to the current session.
