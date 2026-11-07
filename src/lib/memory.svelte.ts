// Live memory: how much RAM the PC has free at this moment (for the memory slider hint).

import { invoke } from '@tauri-apps/api/core';

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const memory = $state({ totalMb: 0, freeMb: 0, at: 0 });

let pending: Promise<void> | null = null;

/** Read the current figures (throttled to once a second). */
export function refreshMemory(): Promise<void> {
  if (!inTauri) return Promise.resolve();
  if (pending) return pending;
  if (Date.now() - memory.at < 1000) return Promise.resolve();
  pending = invoke<{ totalMb: number; freeMb: number }>('memory_status')
    .then((m) => {
      memory.totalMb = m.totalMb;
      memory.freeMb = m.freeMb;
      memory.at = Date.now();
    })
    .catch(() => {
      /* keep the last reading */
    })
    .finally(() => {
      pending = null;
    });
  return pending;
}
