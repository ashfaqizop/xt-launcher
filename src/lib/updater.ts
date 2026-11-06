import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateResult =
  | { kind: 'latest' }
  | { kind: 'available'; version: string; install: (onProgress?: (percent: number) => void) => Promise<void> }
  | { kind: 'error'; message: string };

/** Check GitHub Releases for a newer XT Launcher. Never throws. */
export async function checkForUpdates(): Promise<UpdateResult> {
  try {
    const update = await check({ timeout: 8000 });
    if (!update) return { kind: 'latest' };
    return {
      kind: 'available',
      version: update.version,
      install: async (onProgress) => {
        let total = 0;
        let received = 0;
        await update.downloadAndInstall((e) => {
          if (e.event === 'Started') total = e.data.contentLength ?? 0;
          if (e.event === 'Progress') {
            received += e.data.chunkLength;
            if (total > 0) onProgress?.(Math.min(100, (received / total) * 100));
          }
        });
        await relaunch();
      },
    };
  } catch (err) {
    return { kind: 'error', message: err instanceof Error ? err.message : String(err) };
  }
}
