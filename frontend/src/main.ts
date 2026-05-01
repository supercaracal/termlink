import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';

const ENCODER = new TextEncoder();

interface SessionInfo {
  id: string;
  name: string;
  created_at: number; // unix seconds
}

// ── DOM refs ──────────────────────────────────────────────────────────────────
const sessionListView = document.getElementById('session-list-view')!;
const terminalView = document.getElementById('terminal-view')!;
const sessionListEl = document.getElementById('session-list')!;
const emptyMsg = document.getElementById('empty-msg')!;
const newSessionBtn = document.getElementById(
  'new-session-btn',
) as HTMLButtonElement;
const sessionNameInput = document.getElementById(
  'session-name-input',
) as HTMLInputElement;
const backBtn = document.getElementById('back-btn')!;
const sessionNameLabel = document.getElementById('session-name-label')!;
const statusDot = document.getElementById('status-dot')!;
const statusText = document.getElementById('status-text')!;
const serverDot = document.getElementById('server-dot')!;
const serverText = document.getElementById('server-text')!;
const termContainer = document.getElementById('terminal-container')!;

// ── xterm setup ───────────────────────────────────────────────────────────────
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
term.open(termContainer);

// ── State ─────────────────────────────────────────────────────────────────────
let ws: WebSocket | null = null;
let currentSessionId: string | null = null;
let refreshTimer: ReturnType<typeof setInterval> | null = null;

// ── URL routing ───────────────────────────────────────────────────────────────
function sessionIdFromPath(): string | null {
  const match = location.pathname.match(/^\/sessions\/([^/]+)$/);
  return match ? match[1] : null;
}

// ── Views ─────────────────────────────────────────────────────────────────────
function showSessionList(push = true) {
  if (push) history.pushState(null, '', '/');

  currentSessionId = null;
  disconnectWs();

  terminalView.style.display = 'none';
  sessionListView.style.display = 'flex';

  serverDot.className = '';
  serverText.textContent = 'Checking…';
  newSessionBtn.disabled = false;

  loadSessions();
  refreshTimer = setInterval(loadSessions, 5000);
}

function showTerminal(session: SessionInfo, push = true) {
  if (push)
    history.pushState({ sessionId: session.id }, '', `/sessions/${session.id}`);

  if (refreshTimer !== null) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }

  sessionListView.style.display = 'none';
  terminalView.style.display = 'flex';
  sessionNameLabel.textContent = session.name;

  currentSessionId = session.id;
  term.reset();
  fitAddon.fit();
  connectWs(session.id);
}

// ── Session API ───────────────────────────────────────────────────────────────
function setServerStatus(online: boolean) {
  serverDot.className = online ? 'online' : 'offline';
  serverText.textContent = online ? 'Connected' : 'Server offline';
  newSessionBtn.disabled = !online;
}

async function loadSessions() {
  try {
    const res = await fetch('/sessions');
    const sessions: SessionInfo[] = await res.json();
    setServerStatus(true);
    renderSessionList(sessions);
  } catch {
    setServerStatus(false);
  }
}

function renderSessionList(sessions: SessionInfo[]) {
  if (sessions.length === 0) {
    emptyMsg.style.display = 'block';
    // remove old cards
    for (const card of sessionListEl.querySelectorAll('.session-card')) {
      card.remove();
    }
    return;
  }

  emptyMsg.style.display = 'none';

  // Build id→card map of existing cards for diffing
  const existing = new Map<string, Element>();
  for (const card of sessionListEl.querySelectorAll<HTMLElement>(
    '.session-card',
  )) {
    existing.set(card.dataset.id!, card);
  }

  const ids = new Set(sessions.map((s) => s.id));

  // Remove stale cards
  for (const [id, el] of existing) {
    if (!ids.has(id)) el.remove();
  }

  // Add/update cards in order
  for (const session of sessions) {
    if (existing.has(session.id)) continue; // already rendered

    const card = document.createElement('div');
    card.className = 'session-card';
    card.dataset.id = session.id;

    const meta = document.createElement('div');
    meta.className = 'session-meta';

    const nameEl = document.createElement('span');
    nameEl.className = 'session-name';
    nameEl.textContent = session.name;

    const timeEl = document.createElement('span');
    timeEl.className = 'session-time';
    timeEl.textContent = new Date(
      session.created_at * 1000,
    ).toLocaleTimeString();

    meta.append(nameEl, timeEl);

    const btn = document.createElement('button');
    btn.className = 'attach-btn';
    btn.textContent = 'Attach';
    btn.addEventListener('click', () => showTerminal(session));

    card.append(meta, btn);
    sessionListEl.appendChild(card);
  }
}

async function createSession() {
  newSessionBtn.disabled = true;
  const name = sessionNameInput.value.trim();
  const url = name ? `/sessions?name=${encodeURIComponent(name)}` : '/sessions';
  try {
    const res = await fetch(url, { method: 'POST' });
    if (!res.ok) throw new Error('Failed to create session');
    const session: SessionInfo = await res.json();
    sessionNameInput.value = '';
    showTerminal(session);
  } catch (e) {
    console.error(e);
    newSessionBtn.disabled = false;
  }
}

// ── WebSocket ─────────────────────────────────────────────────────────────────
function setStatus(connected: boolean, message: string) {
  statusDot.className = connected ? '' : 'disconnected';
  statusText.textContent = message;
}

function disconnectWs() {
  if (ws) {
    ws.onclose = null;
    ws.close();
    ws = null;
  }
}

function connectWs(sessionId: string) {
  disconnectWs();
  setStatus(false, 'Connecting…');

  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(`${protocol}//${location.host}/ws?session=${sessionId}`);
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
    // Only reconnect if we're still on this session
    if (currentSessionId !== sessionId) return;
    setStatus(false, 'Disconnected — reconnecting in 3s…');
    term.write('\r\n\x1b[31m[disconnected]\x1b[0m\r\n');
    setTimeout(async () => {
      if (currentSessionId !== sessionId) return;
      // If the session was removed on the server side (e.g. shell exit), go back to the list
      try {
        const sessions: SessionInfo[] = await fetch('/sessions').then((r) =>
          r.json(),
        );
        if (!sessions.find((s) => s.id === sessionId)) {
          term.write('\r\n\x1b[33m[session ended]\x1b[0m\r\n');
          setTimeout(() => {
            if (currentSessionId === sessionId) showSessionList();
          }, 1500);
          return;
        }
      } catch {
        /* server unreachable — fall through to reconnect */
      }
      connectWs(sessionId);
    }, 3000);
  };

  ws.onerror = () => {
    ws?.close();
  };
}

function sendResize() {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(
      JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }),
    );
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
resizeObserver.observe(termContainer);

// ── Event listeners ───────────────────────────────────────────────────────────
newSessionBtn.addEventListener('click', createSession);
backBtn.addEventListener('click', () => showSessionList());
sessionNameInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') createSession();
});

window.addEventListener('popstate', async () => {
  const id = sessionIdFromPath();
  if (id) {
    try {
      const sessions: SessionInfo[] = await fetch('/sessions').then((r) =>
        r.json(),
      );
      const session = sessions.find((s) => s.id === id);
      if (session) {
        showTerminal(session, false);
        return;
      }
    } catch {
      /* fall through to session list */
    }
  }
  showSessionList(false);
});

// ── Bootstrap ─────────────────────────────────────────────────────────────────
async function bootstrap() {
  const id = sessionIdFromPath();
  if (id) {
    try {
      const sessions: SessionInfo[] = await fetch('/sessions').then((r) =>
        r.json(),
      );
      const session = sessions.find((s) => s.id === id);
      if (session) {
        showTerminal(session, false);
        return;
      }
    } catch {
      /* fall through to session list */
    }
  }
  showSessionList(false);
}

bootstrap();
