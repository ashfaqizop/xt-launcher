// Bridge to the Rust engine: launch/stop, progress, live game log and crash info.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { currentAccount, currentProfile } from './data.svelte';
import { go } from './nav.svelte';
import { settings } from './settings.svelte';
import { showToast } from './toast.svelte';
import { versions } from './versions.svelte';

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

export const engine = $state({
  phase: 'idle' as Phase,
  /** download stage currently running: client | libraries | assets | java */
  stage: '',
  label: 'Preparing Minecraft',
  percent: 0,
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
  state: 'installing' | 'running' | 'exited' | 'error';
  message: string | null;
  code: number | null;
  crash: CrashInfo | null;
}

async function onState(p: StatePayload) {
  const win = getCurrentWindow();
  switch (p.state) {
    case 'installing':
      engine.phase = 'installing';
      engine.label = p.message ?? 'Preparing Minecraft';
      engine.percent = 0;
      break;
    case 'running':
      engine.phase = 'running';
      if (settings.afterLaunch === 'tray') void win.minimize();
      else if (settings.afterLaunch === 'close') void win.close();
      break;
    case 'exited':
      engine.phase = 'idle';
      flushLogs();
      void win.unminimize();
      void win.setFocus();
      if (p.crash) {
        engine.crash = p.crash;
        showToast(p.crash.title, 4000);
        go('logs');
      }
      break;
    case 'error':
      engine.phase = 'idle';
      engine.error = p.message ?? 'Something went wrong';
      showToast("Couldn't start the game", 4000);
      void win.unminimize();
      go('logs');
      break;
  }
}

const STAGE_LABEL: Record<string, string> = {
  client: 'Downloading Minecraft',
  libraries: 'Downloading libraries',
  assets: 'Downloading game assets',
  java: 'Downloading Java',
};

export async function initEngine() {
  if (!inTauri) return;

  await listen<{ stage: string; done: number; total: number }>('engine-progress', (e) => {
    const { stage, done, total } = e.payload;
    engine.stage = stage;
    engine.label = STAGE_LABEL[stage] ?? 'Preparing Minecraft';
    engine.percent = total > 0 ? Math.min(100, (done / total) * 100) : 0;
  });
  await listen<StatePayload>('game-state', (e) => void onState(e.payload));
  await listen<LogLine>('game-log', (e) => {
    pending.push(e.payload);
  });
  flushTimer ??= setInterval(flushLogs, 120);

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
  engine.phase = 'installing';
  engine.percent = 0;
  engine.label = 'Preparing Minecraft';

  try {
    await invoke('launch_game', {
      req: {
        profileId: profile.id,
        loader: profile.loader,
        mc: profile.mc,
        account: { name: account.name, kind: account.type, id: account.id },
        msClientId: settings.msClientId || null,
        ramMb: profile.ramMb ?? (settings.ramAuto ? null : settings.ramMb),
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
    engine.phase = 'idle';
    showToast(String(err));
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
    await invoke('open_dir', { kind, profileId: currentProfile().id });
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
