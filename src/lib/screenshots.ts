// A profile's screenshots: listing, thumbnails, full images, copy to clipboard, delete.
// Thumbnails are decoded at a small size and cached, so a strip of them stays light.

import { invoke } from '@tauri-apps/api/core';

export interface Screenshot {
  name: string;
  /** Last modified, epoch ms */
  modified: number;
  size: number;
}

export const listScreenshots = (profileId: string, limit?: number) =>
  invoke<Screenshot[]>('list_screenshots', { profileId, limit: limit ?? null });

// ---- thumbnails ---------------------------------------------------------------

const MAX_CACHED = 160;
const thumbs = new Map<string, Promise<string>>();
let active = 0;
const waiters: (() => void)[] = [];

async function takeSlot() {
  if (active >= 3) await new Promise<void>((resolve) => waiters.push(resolve));
  active++;
}
function freeSlot() {
  active--;
  waiters.shift()?.();
}

async function bytesOf(profileId: string, name: string): Promise<Blob> {
  const buffer = await invoke<ArrayBuffer>('read_screenshot', { profileId, name });
  return new Blob([buffer]);
}

function toBlob(canvas: HTMLCanvasElement, type: string, quality?: number): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('could not encode the image'))), type, quality),
  );
}

async function makeThumbnail(profileId: string, name: string): Promise<string> {
  await takeSlot();
  try {
    const blob = await bytesOf(profileId, name);
    const bitmap = await createImageBitmap(blob, { resizeWidth: 520, resizeQuality: 'medium' });
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    canvas.getContext('2d')!.drawImage(bitmap, 0, 0);
    bitmap.close();
    return URL.createObjectURL(await toBlob(canvas, 'image/webp', 0.82));
  } finally {
    freeSlot();
  }
}

/** A small object-URL picture for one screenshot (cached). */
export function thumbnail(profileId: string, shot: Screenshot): Promise<string> {
  const key = `${profileId}/${shot.name}/${shot.modified}`;
  let hit = thumbs.get(key);
  if (!hit) {
    hit = makeThumbnail(profileId, shot.name);
    thumbs.set(key, hit);
    hit.catch(() => thumbs.delete(key));
    if (thumbs.size > MAX_CACHED) {
      const oldest = thumbs.keys().next().value as string | undefined;
      if (oldest) {
        const old = thumbs.get(oldest);
        thumbs.delete(oldest);
        void old?.then((url) => URL.revokeObjectURL(url)).catch(() => {});
      }
    }
  }
  return hit;
}

/** The whole picture as an object URL; revoke it when done. */
export async function fullImage(profileId: string, name: string): Promise<string> {
  return URL.createObjectURL(await bytesOf(profileId, name));
}

/** Put the screenshot on the clipboard as an image. */
export async function copyScreenshot(profileId: string, name: string): Promise<void> {
  const bitmap = await createImageBitmap(await bytesOf(profileId, name));
  const canvas = document.createElement('canvas');
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0);
  bitmap.close();
  const png = await toBlob(canvas, 'image/png');
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': png })]);
}

export const openScreenshot = (profileId: string, name: string) =>
  invoke<void>('open_screenshot', { profileId, name });

export const deleteScreenshot = (profileId: string, name: string) =>
  invoke<void>('delete_screenshot', { profileId, name });
