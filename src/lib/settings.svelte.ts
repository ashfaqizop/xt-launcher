// UI + game settings. Persisted in localStorage for now; the game-related fields are
// wired to the Rust engine in the next phase.

export interface Settings {
  // Launcher
  themeMode: 'system' | 'light' | 'dark';
  dynamicColor: boolean;
  /** id of a gallery wallpaper, or 'custom' */
  wallpaper: string;
  blur: number; // px
  dim: number; // percent
  afterLaunch: 'tray' | 'close' | 'keep';
  crashScan: boolean;
  alertRelease: boolean;
  alertSnapshot: boolean;
  // Minecraft
  gameFolder: string;
  winW: number;
  winH: number;
  fullscreen: boolean;
  javaMode: 'auto' | 'custom';
  javaPath: string;
  ramAuto: boolean;
  ramMb: number;
  gpu: 'default' | 'high' | 'low';
  showSnapshots: boolean;
  showOld: boolean;
  /** Azure application (client) id for Microsoft sign-in */
  msClientId: string;
}

export const DEFAULTS: Settings = {
  themeMode: 'system',
  dynamicColor: true,
  wallpaper: 'xt',
  blur: 6,
  dim: 25,
  afterLaunch: 'tray',
  crashScan: true,
  alertRelease: false,
  alertSnapshot: false,
  gameFolder: '%APPDATA%\\XT Launcher',
  winW: 925,
  winH: 530,
  fullscreen: false,
  javaMode: 'auto',
  javaPath: '',
  ramAuto: true,
  ramMb: 4096,
  gpu: 'default',
  showSnapshots: false,
  showOld: false,
  msClientId: '',
};

const KEY = 'xt.settings.v1';

function load(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...DEFAULTS, ...JSON.parse(raw) };
  } catch {
    /* storage unavailable: fall back to defaults */
  }
  return { ...DEFAULTS };
}

export const settings: Settings = $state(load());

$effect.root(() => {
  $effect(() => {
    const json = JSON.stringify(settings);
    try {
      localStorage.setItem(KEY, json);
    } catch {
      /* ignore */
    }
  });
});

export function resetSettings() {
  Object.assign(settings, DEFAULTS);
}
