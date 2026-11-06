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

// ---------------------------------------------------------------------------
// Intro: logo + "XT Launcher" blur in at the centre, hold, then zoom out to the
// top-left. The update check runs underneath it; nothing else is shown until the
// brand has settled, and the splash is never closed before the intro finishes.
// ---------------------------------------------------------------------------

const INTRO_START_SCALE = 1.45;
const INTRO_FADE_IN_MS = 650;
const INTRO_HOLD_MS = 200;
const INTRO_ZOOM_OUT_MS = 600;

async function playIntro(): Promise<void> {
  const brand = $('brand');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const first: Animation[] = [];

  try {
    if (reduced) return;

    // Measure after the font and logo are ready so the centring is exact.
    await Promise.race([
      Promise.all([
        document.fonts.load('700 22px "Bricolage Grotesque Variable"'),
        (brand.querySelector('img') as HTMLImageElement).decode().catch(() => {}),
      ]),
      sleep(400),
    ]);

    const r = brand.getBoundingClientRect();
    const dx = innerWidth / 2 - (r.width * INTRO_START_SCALE) / 2 - r.x;
    const dy = innerHeight / 2 - (r.height * INTRO_START_SCALE) / 2 - r.y;
    const start = `translate(${dx}px, ${dy}px) scale(${INTRO_START_SCALE})`;

    const blurIn = brand.animate(
      [
        { transform: start, filter: 'blur(18px)', opacity: 0 },
        { transform: start, filter: 'blur(0px)', opacity: 1 },
      ],
      { duration: INTRO_FADE_IN_MS, easing: 'cubic-bezier(0.2, 0.7, 0.2, 1)', fill: 'forwards' },
    );
    first.push(blurIn);
    await blurIn.finished;
    await sleep(INTRO_HOLD_MS);

    const zoomOut = brand.animate(
      [{ transform: start }, { transform: 'translate(0, 0) scale(1)' }],
      { duration: INTRO_ZOOM_OUT_MS, easing: 'cubic-bezier(0.3, 0, 0, 1)', fill: 'forwards' },
    );
    first.push(zoomOut);
    await zoomOut.finished;
  } catch (err) {
    console.warn('Intro skipped:', err);
  } finally {
    // Reveal the rest, then drop the animations (the .ready styles match their end state).
    document.body.classList.add('ready');
    first.forEach((a) => a.cancel());
  }
}

// Dev-only: ?demo=<state> previews a static state (add &intro to include the intro)
const params = import.meta.env.DEV ? new URLSearchParams(location.search) : null;
const demo = params?.get('demo');
const skipIntro = !!demo && !params?.has('intro');
if (skipIntro) document.body.classList.add('ready');
const intro: Promise<void> = skipIntro ? Promise.resolve() : playIntro();

async function launchMain() {
  await intro; // never close the splash mid-animation
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

if (demo && DEMOS[demo]) {
  render(DEMOS[demo]);
} else {
  void run();
}
