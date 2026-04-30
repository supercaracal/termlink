import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

const RECONNECT_DELAY_MS = 3000;
const ENCODER = new TextEncoder();

const term = new Terminal({
  cursorBlink: true,
  fontSize: 14,
  fontFamily: '"Menlo", "Monaco", "Courier New", monospace',
  theme: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    selectionBackground: '#264f78',
  },
});

const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.loadAddon(new WebLinksAddon());

const container = document.getElementById('terminal-container')!;
const statusDot = document.getElementById('status-dot')!;
const statusText = document.getElementById('status-text')!;

term.open(container);
fitAddon.fit();

function setStatus(connected: boolean, message: string) {
  statusDot.className = connected ? '' : 'disconnected';
  statusText.textContent = message;
}

let ws: WebSocket | null = null;

function connect() {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(`${protocol}//${location.host}/ws`);
  ws.binaryType = 'arraybuffer';

  ws.onopen = () => {
    setStatus(true, 'Connected');
    sendResize();
  };

  ws.onmessage = (e: MessageEvent) => {
    if (e.data instanceof ArrayBuffer) {
      term.write(new Uint8Array(e.data));
    }
  };

  ws.onclose = () => {
    setStatus(false, `Disconnected — reconnecting in ${RECONNECT_DELAY_MS / 1000}s…`);
    term.write('\r\n\x1b[31m[disconnected]\x1b[0m\r\n');
    setTimeout(connect, RECONNECT_DELAY_MS);
  };

  ws.onerror = () => {
    ws?.close();
  };
}

function sendResize() {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
  }
}

term.onData((data: string) => {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(ENCODER.encode(data));
  }
});

const resizeObserver = new ResizeObserver(() => {
  fitAddon.fit();
  sendResize();
});
resizeObserver.observe(container);

connect();
