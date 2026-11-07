// What this PC has (from the Rust engine): installed memory, graphics cards, Java installs.

import { invoke } from '@tauri-apps/api/core';
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface Gpu {
  name: string;
  integrated: boolean;
}
export interface JavaInstall {
  path: string;
  version: string;
  major: number;
  vendor: string;
  managed: boolean;
}

export const system = $state({
  loaded: false,
  totalMemoryMb: 0,
  gpus: [] as Gpu[],
  javas: [] as JavaInstall[],
});

let pending: Promise<void> | null = null;

/** Detect once per session (cheap to call repeatedly). */
export function loadSystem(force = false): Promise<void> {
  if (!inTauri) return Promise.resolve();
  if (pending && !force) return pending;
  pending = invoke<{ totalMemoryMb: number; gpus: Gpu[]; javas: JavaInstall[] }>('system_info')
    .then((info) => {
      system.totalMemoryMb = info.totalMemoryMb;
      system.gpus = info.gpus;
      system.javas = info.javas;
      system.loaded = true;
    })
    .catch(() => {
      system.loaded = true;
    });
  return pending;
}

/** Highest memory the slider offers: installed RAM minus room for Windows. */
export function maxGameMemoryGiB(): number {
  if (!system.totalMemoryMb) return 16;
  return Math.max(2, Math.floor((system.totalMemoryMb / 1024 - 1.5) * 2) / 2);
}

/** Memory used when "Auto" is on, scaled to the installed RAM. */
export function autoMemoryMb(): number {
  const gib = system.totalMemoryMb / 1024;
  if (!gib) return 4096;
  if (gib <= 4.5) return 1536;
  if (gib <= 8.5) return 3072;
  if (gib <= 16.5) return 4096;
  if (gib <= 32.5) return 6144;
  return 8192;
}

/** A lower memory limit for small PCs (8 GB or less), so Windows keeps room to breathe. */
export function lowEndMemoryMb(): number {
  const gib = system.totalMemoryMb / 1024;
  if (!gib || gib > 6.5) return 2560;
  if (gib > 4.5) return 2048;
  return 1536;
}

export const gib = (mb: number) => {
  const v = mb / 1024;
  return Number.isInteger(v) ? `${v}` : v.toFixed(1);
};
