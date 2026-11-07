// Bridge to the Rust engine: launch/stop, progress, live game log and crash info.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { currentAccount, currentProfile, recordPlaytime } from './data.svelte';
import { go, goCreateProfile } from './nav.svelte';
import { settings } from './settings.svelte';
import { showToast } from './toast.svelte';
import { versions } from './versions.svelte';
import { autoMemoryMb, loadSystem } from './system.svelte';

export interface CrashInfo {
  title: string;
  hint: string;
  report: string | null;
}
export interface LogLine {
  /** out / err: the game; xt: messages from the launcher itself */
  stream: 'out' | 'err' | 'xt';
  line: string;
}

type Phase = 'idle' | 'installing' | 'running';

/** Pipeline steps, in the order they normally happen. */
export const STEPS = ['mods', 'loader', 'client', 'libraries', 'assets', 'java', 'launch'] as const;
export type Step = (typeof STEPS)[number];
export const STEP_LABEL: Record<Step, string> = {
  mods: 'Mods',
  loader: 'Loader',
  client: 'Minecraft',
  libraries: 'Libraries',
  assets: 'Assets',
  java: 'Java',
  launch: 'Launch',
};

export interface LastSession {
  profileName: string;
  playedMs: number;
  code: number | null;
  crash: CrashInfo | null;
  error: string;
  cancelled: boolean;
  at: number;
}

export const engine = $state({
  phase: 'idle' as Phase,
  /** Profile being prepared / played */
  profileName: '',
  profileId: '',
  /** Current pipeline step and its stats */
  stage: '' as Step | '',
  label: 'Preparing',
  done: 0,
  total: 0,
  fetched: 0,
  bytes: 0,
  /** Bytes per second, smoothed */
  speed: 0,
  percent: 0,
  /** Steps seen during this launch, in order */
  steps: [] as Step[],
  startedAt: 0,
  runningSince: 0,
  cancelling: false,
  /** Whether the launch ribbon is currently on screen (set by the ribbon) */
  ribbonOpen: false,
  /** Summary of the session that just ended (shown in the ribbon) */
  last: null as LastSession | null,
  logs: [] as LogLine[],
  crash: null as CrashInfo | null,
  error: '',
  dataDir: '',
});

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const MAX_LINES = 5000;
let pending: LogLine[] = [];
let flushTimer: ReturnType<typeof setInterval> | undefined;

function flushLogs() {
  if (!pending.length) return;
  const next = engine.logs.concat(pending);
  pending = [];
  engine.logs = next.length > MAX_LINES ? next.slice(next.length - MAX_LINES) : next;
}

interface StatePayload {
  state: 'installing' | 'running' | 'exited' | 'error' | 'cancelled';
  message: string | null;
  code: number | null;
  crash: CrashInfo | null;
}

function markStep(step: Step) {
  if (!engine.steps.includes(step)) engine.steps = [...engine.steps, step];
  engine.stage = step;
}

function finishSession(partial: Partial<LastSession>) {
  const played = engine.runningSince ? Date.now() - engine.runningSince : 0;
  engine.last = {
    profileName: engine.profileName,
    playedMs: played,
    code: null,
    crash: null,
    error: '',
    cancelled: false,
    at: Date.now(),
    ...partial,
  };
  if (played > 0) recordPlaytime(engine.profileId, played);
  engine.phase = 'idle';
  engine.runningSince = 0;
  engine.cancelling = false;
}

async function onState(p: StatePayload) {
  const win = getCurrentWindow();
  switch (p.state) {
    case 'installing':
      engine.phase = 'installing';
      if (p.message === 'Launching Minecraft') {
        markStep('launch');
        engine.label = 'Launching';
        engine.percent = 0;
        engine.total = 0;
      } else if (p.message === 'Updating mods') {
        markStep('mods');
        engine.label = 'Updating mods';
      } else if (p.message) {
        engine.label = p.message;
      }
      break;
    case 'running':
      engine.phase = 'running';
      engine.runningSince = Date.now();
      if (settings.afterLaunch === 'tray') void win.minimize();
      else if (settings.afterLaunch === 'close') void win.close();
      break;
    case 'exited':
      flushLogs();
      if (p.crash) engine.crash = p.crash;
      finishSession({ code: p.code, crash: p.crash });
      void win.unminimize();
      void win.setFocus();
      break;
    case 'error':
      engine.error = p.message ?? 'Something went wrong';
      finishSession({ error: engine.error });
      void win.unminimize();
      break;
    case 'cancelled':
      finishSession({ cancelled: true });
      break;
  }
}

const STAGE_LABEL: Record<string, string> = {
  client: 'Minecraft',
  libraries: 'libraries',
  assets: 'game assets',
  java: 'Java',
  mods: 'mods',
  loader: 'loader',
};

let lastSample = { t: 0, bytes: 0, stage: '' };

interface ProgressPayload {
  stage: string;
  done: number;
  total: number;
  label: string;
  fetched: number;
  bytes: number;
}

function onProgress(p: ProgressPayload) {
  if (engine.phase !== 'installing') return;
  const step = (STEPS as readonly string[]).includes(p.stage) ? (p.stage as Step) : null;
  if (step) markStep(step);
  engine.done = p.done;
  engine.total = p.total;
  engine.fetched = p.fetched;
  engine.bytes = p.bytes;
  engine.percent = p.total > 0 ? Math.min(100, (p.done / p.total) * 100) : 0;

  const what = STAGE_LABEL[p.stage] ?? 'files';
  if (engine.cancelling) engine.label = 'Cancelling';
  else if (p.stage === 'loader') engine.label = p.label;
  else if (p.fetched > 0) engine.label = 'Downloading ' + what;
  else engine.label = 'Checking ' + what;

  // Download speed, smoothed over samples at least 400 ms apart
  const now = performance.now();
  if (lastSample.stage !== p.stage || p.bytes < lastSample.bytes) {
    lastSample = { t: now, bytes: p.bytes, stage: p.stage };
    engine.speed = 0;
  } else if (now - lastSample.t >= 400) {
    const rate = ((p.bytes - lastSample.bytes) * 1000) / (now - lastSample.t);
    engine.speed = engine.speed ? engine.speed * 0.6 + rate * 0.4 : rate;
    lastSample = { t: now, bytes: p.bytes, stage: p.stage };
  }
}

export async function initEngine() {
  if (!inTauri) return;

  await listen<ProgressPayload>('engine-progress', (e) => onProgress(e.payload));
  await listen<StatePayload>('game-state', (e) => void onState(e.payload));
  await listen<LogLine>('game-log', (e) => {
    pending.push(e.payload);
  });
  flushTimer ??= setInterval(flushLogs, 120);
  void loadSystem();

  try {
    const info = await invoke<{ dataDir: string }>('engine_info');
    engine.dataDir = info.dataDir;
  } catch {
    /* non-fatal */
  }
  try {
    const list = await invoke<{ latestRelease: string }>('list_versions', {
      snapshots: false,
      old: false,
    });
    versions.latestRelease = list.latestRelease;
  } catch {
    /* offline: the profile subtitle falls back to "Latest" */
  }
}

export async function play() {
  const account = currentAccount();
  const profile = currentProfile();
  if (!profile) {
    showToast('Create a profile first');
    goCreateProfile();
    return;
  }
  if (!account) {
    showToast('Add an account to play');
    go('accounts');
    return;
  }
  if (!inTauri) {
    showToast('Launching works in the desktop app');
    return;
  }
  engine.crash = null;
  engine.error = '';
  engine.logs = [];
  pending = [];
  engine.last = null;
  engine.phase = 'installing';
  engine.profileName = profile.name;
  engine.profileId = profile.id;
  engine.steps = [];
  engine.stage = '';
  engine.done = engine.total = engine.fetched = engine.bytes = 0;
  engine.speed = 0;
  engine.percent = 0;
  engine.label = 'Preparing';
  engine.startedAt = Date.now();
  engine.cancelling = false;

  try {
    await invoke('launch_game', {
      req: {
        profileId: profile.id,
        loader: profile.loader,
        loaderVersion: profile.loaderVersion || null,
        mc: profile.mc,
        account: { name: account.name, kind: account.type, id: account.id },
        msClientId: settings.msClientId || null,
        ramMb: profile.ramMb ?? (settings.ramAuto ? autoMemoryMb() : settings.ramMb),
        jvmArgs: profile.jvmArgs || null,
        xtTier: profile.kind === 'custom' ? null : profile.kind,
        jvmPreset: profile.kind !== 'custom' || profile.zgc ? 'zgc' : null,
        width: settings.winW,
        height: settings.winH,
        fullscreen: settings.fullscreen,
        javaPath: settings.javaMode === 'custom' ? settings.javaPath : null,
        gpu: settings.gpu,
      },
    });
  } catch (err) {
    engine.error = String(err);
    finishSession({ error: engine.error });
  }
}

/** Cancel a launch that is still preparing. */
export async function cancelLaunch() {
  engine.cancelling = true;
  engine.label = 'Cancelling';
  try {
    await invoke('cancel_launch');
  } catch {
    /* ignore */
  }
}

export async function stop() {
  try {
    await invoke('kill_game');
  } catch (err) {
    showToast(String(err));
  }
}

export async function openFolder(kind: 'root' | 'game' | 'mods') {
  if (!inTauri) {
    showToast('Folders open in the desktop app');
    return;
  }
  try {
    await invoke('open_dir', { kind, profileId: currentProfile()?.id ?? null });
  } catch (err) {
    showToast(String(err));
  }
}

/** Load the Minecraft version list for the profile editor. */
export async function loadVersionList(snapshots: boolean) {
  if (!inTauri) return;
  try {
    const list = await invoke<{ latestRelease: string; versions: { id: string; kind: string }[] }>(
      'list_versions',
      { snapshots, old: false },
    );
    versions.latestRelease = list.latestRelease;
    versions.list = list.versions;
  } catch {
    /* offline: the editor keeps what it has */
  }
}

// Dev-only hook so UI states can be previewed in a browser
if (import.meta.env.DEV && typeof window !== "undefined") {
  (window as unknown as { __xt: unknown }).__xt = { engine };
}
