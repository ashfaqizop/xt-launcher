import '@fontsource-variable/bricolage-grotesque/wght.css';
import '@m3e/web/theme';
import '@m3e/web/loading-indicator';
import '@m3e/web/progress-indicator';
import './styles/base.css';
import './splash.css';

import { invoke } from '@tauri-apps/api/core';
import { check, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

type Indicator = 'wavy' | 'progress' | 'loading';

interface View {
  status: string;
  detail?: string;
  indicator: Indicator;
  /** 0-100, only used by the `progress` indicator */
  percent?: number;
}

type ProgressEl = HTMLElement & { value: number };

const $ = (id: string) => document.getElementById(id) as HTMLElement;
const statusEl = $('status');
const detailEl = $('detail');
const indicators: Record<Indicator, HTMLElement> = {
  wavy: $('ind-wavy'),
  progress: $('ind-progress'),
  loading: $('ind-loading'),
};

function setText(el: HTMLElement, text: string) {
  if (el.textContent === text) return;
  el.textContent = text;
  el.classList.remove('swap');
  void el.offsetWidth; // restart the animation
  el.classList.add('swap');
}

function render(view: View) {
  setText(statusEl, view.status);
  setText(detailEl, view.detail ?? '');

  for (const [name, el] of Object.entries(indicators)) {
    el.hidden = name !== view.indicator;
  }
  if (view.indicator === 'progress') {
    (indicators.progress as ProgressEl).value = view.percent ?? 0;
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

const mb = (bytes: number) => (bytes / 1024 / 1024).toFixed(1);

async function launchMain() {
  render({ status: 'Launching XT', indicator: 'loading' });
  try {
    await invoke('finish_splash');
  } catch (err) {
    console.error('finish_splash failed:', err);
  }
}

// ---------------------------------------------------------------------------
// Flow: connect -> check -> (download -> install -> restart) | launch
// ---------------------------------------------------------------------------

const CHECK_TIMEOUT_MS = 8000;
/** If the check is still pending after this long, say what is probably happening. */
const SLOW_CHECK_MS = 700;

async function run() {
  render({ status: 'Connecting to GitHub', indicator: 'wavy' });
  const slow = setTimeout(
    () => render({ status: 'Checking For Updates', indicator: 'wavy' }),
    SLOW_CHECK_MS,
  );

  let update;
  try {
    update = await check({ timeout: CHECK_TIMEOUT_MS });
  } catch (err) {
    // Offline, GitHub unreachable, or no release published yet: launch anyway.
    console.warn('Update check failed:', err);
    clearTimeout(slow);
    render({
      status: "Can't reach GitHub",
      detail: 'Continuing without updates',
      indicator: 'wavy',
    });
    await sleep(900);
    return launchMain();
  }
  clearTimeout(slow);

  if (!update) return launchMain();

  // Update available
  let total = 0;
  let received = 0;
  const title = 'Downloading Updates';
  const label = `v${update.version}`;
  render({ status: title, detail: label, indicator: 'wavy' });

  try {
    await update.downloadAndInstall((event: DownloadEvent) => {
      switch (event.event) {
        case 'Started':
          total = event.data.contentLength ?? 0;
          break;
        case 'Progress': {
          received += event.data.chunkLength;
          if (total > 0) {
            const percent = Math.min(100, (received / total) * 100);
            render({
              status: title,
              detail: `${label}  ·  ${mb(received)} / ${mb(total)} MB  ·  ${Math.floor(percent)}%`,
              indicator: 'progress',
              percent,
            });
          } else {
            render({ status: title, detail: `${label}  ·  ${mb(received)} MB`, indicator: 'wavy' });
          }
          break;
        }
        case 'Finished':
          render({ status: 'Installing Update', detail: label, indicator: 'wavy' });
          break;
      }
    });
    // On Windows the installer normally ends this process itself. If we are
    // still here, restart into the new version.
    render({ status: 'Restarting XT', indicator: 'loading' });
    await relaunch();
  } catch (err) {
    console.error('Update failed:', err);
    render({
      status: 'Update failed',
      detail: 'Starting the current version',
      indicator: 'wavy',
    });
    await sleep(1400);
    return launchMain();
  }
}

// ---------------------------------------------------------------------------
// Dev-only: ?demo=<state> renders a static state for visual checks in a browser
// ---------------------------------------------------------------------------

const DEMOS: Record<string, View> = {
  connecting: { status: 'Connecting to GitHub', indicator: 'wavy' },
  checking: { status: 'Checking For Updates', indicator: 'wavy' },
  downloading: {
    status: 'Downloading Updates',
    detail: 'v0.2.0  ·  12.4 / 31.0 MB  ·  40%',
    indicator: 'progress',
    percent: 40,
  },
  installing: { status: 'Installing Update', detail: 'v0.2.0', indicator: 'wavy' },
  launching: { status: 'Launching XT', indicator: 'loading' },
  offline: {
    status: "Can't reach GitHub",
    detail: 'Continuing without updates',
    indicator: 'wavy',
  },
};

const demo = import.meta.env.DEV ? new URLSearchParams(location.search).get('demo') : null;
if (demo && DEMOS[demo]) {
  render(DEMOS[demo]);
} else {
  void run();
}
