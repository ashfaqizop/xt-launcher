import { hexFromArgb, sourceColorFromImageBytes } from '@material/material-color-utilities';
import { settings } from './settings.svelte';

import xt from '../assets/wallpapers/xt.webp';
import javaEdition from '../assets/wallpapers/java-edition.webp';
import copperAge from '../assets/wallpapers/copper-age.webp';
import trickyTrials from '../assets/wallpapers/tricky-trials.webp';
import gardenAwakens from '../assets/wallpapers/garden-awakens.webp';
import cavesAndCliffs from '../assets/wallpapers/caves-and-cliffs.webp';

export interface GalleryItem {
  id: string;
  name: string;
  url: string;
  /** Official Minecraft artwork (© Mojang AB / Microsoft) */
  official: boolean;
}

export const GALLERY: GalleryItem[] = [
  { id: 'xt', name: 'XT Ember', url: xt, official: false },
  { id: 'java-edition', name: 'Java Edition', url: javaEdition, official: true },
  { id: 'copper-age', name: 'The Copper Age', url: copperAge, official: true },
  { id: 'tricky-trials', name: 'Tricky Trials', url: trickyTrials, official: true },
  { id: 'garden-awakens', name: 'The Garden Awakens', url: gardenAwakens, official: true },
  { id: 'caves-and-cliffs', name: 'Caves & Cliffs', url: cavesAndCliffs, official: true },
];

export const BRAND_SEED = '#c23a24';

/** Current wallpaper url and the seed colour extracted from it. */
export const DEFAULT_WALLPAPER = 'caves-and-cliffs';

export const wp = $state({ url: cavesAndCliffs, seed: null as string | null, hasCustom: false });

// ---- custom wallpaper storage (IndexedDB, a single blob) ------------------

const DB = 'xt-launcher';
const STORE = 'kv';

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function idbGet(key: string): Promise<Blob | null> {
  try {
    const db = await openDb();
    return await new Promise((resolve, reject) => {
      const r = db.transaction(STORE).objectStore(STORE).get(key);
      r.onsuccess = () => resolve((r.result as Blob) ?? null);
      r.onerror = () => reject(r.error);
    });
  } catch {
    return null;
  }
}

async function idbSet(key: string, value: Blob): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE, 'readwrite');
    tx.objectStore(STORE).put(value, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

// ---- colour extraction ---------------------------------------------------

async function seedFromUrl(url: string): Promise<string> {
  const img = new Image();
  img.src = url;
  await img.decode();
  const w = 64;
  const h = Math.max(1, Math.round((w * img.naturalHeight) / img.naturalWidth));
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
  ctx.drawImage(img, 0, 0, w, h);
  const { data } = ctx.getImageData(0, 0, w, h);
  return hexFromArgb(sourceColorFromImageBytes(data));
}

// ---- public API ----------------------------------------------------------

let token = 0;
let customUrl: string | null = null;

/** Resolve settings.wallpaper to a url and refresh the extracted seed colour. */
export async function applyWallpaper(id: string) {
  const mine = ++token;
  let url = GALLERY.find((g) => g.id === id)?.url ?? cavesAndCliffs;

  if (id === 'custom') {
    const blob = await idbGet('wallpaper');
    if (mine !== token) return;
    if (blob) {
      if (customUrl) URL.revokeObjectURL(customUrl);
      customUrl = URL.createObjectURL(blob);
      url = customUrl;
    } else {
      settings.wallpaper = DEFAULT_WALLPAPER; // the stored image is gone
      return;
    }
  }

  wp.url = url;
  try {
    const seed = await seedFromUrl(url);
    if (mine === token) wp.seed = seed;
  } catch {
    if (mine === token) wp.seed = null;
  }
}

export async function refreshCustomFlag() {
  wp.hasCustom = (await idbGet('wallpaper')) !== null;
}

/** Downscale a user-picked image, store it, and switch to it. */
export async function setCustomWallpaper(file: File) {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, 2560 / bitmap.width);
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const blob: Blob = await new Promise((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('encode failed'))), 'image/webp', 0.85),
  );
  await idbSet('wallpaper', blob);
  wp.hasCustom = true;
  if (settings.wallpaper === 'custom') await applyWallpaper('custom');
  else settings.wallpaper = 'custom';
}
