import '@fontsource-variable/bricolage-grotesque/wght.css';
import '@m3e/web/theme';
import '@m3e/web/loading-indicator';
import './styles/base.css';
import './splash.css';

import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const statusEl = document.getElementById('status')!;
const setStatus = (text: string) => {
  statusEl.textContent = text;
};

const withTimeout = <T>(p: Promise<T>, ms: number): Promise<T> =>
  Promise.race([p, new Promise<T>((_, rej) => setTimeout(() => rej(new Error('timeout')), ms))]);

async function run() {
  try {
    setStatus('Connecting to GitHub');
    const update = await withTimeout(check(), 8000);

    if (update) {
      setStatus('Downloading Updates');
      await update.downloadAndInstall();
      setStatus('Restarting XT');
      await relaunch();
      return;
    }
  } catch (err) {
    // Offline / GitHub unreachable / no release yet: skip the update and launch anyway.
    console.warn('Update check skipped:', err);
  }

  setStatus('Launching XT');
  try {
    await invoke('finish_splash');
  } catch (err) {
    console.error(err);
  }
}

void run();
