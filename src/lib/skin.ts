// Ely.by skins by player name, cached for the session. Resolves to null when the
// name has no skin (or the network is unavailable).

export interface Skin {
  url: string;
  /** 64 for modern skins, 32 for legacy 64x32 ones */
  height: number;
}

const cache = new Map<string, Promise<Skin | null>>();

export function loadSkin(name: string): Promise<Skin | null> {
  const n = name.trim();
  if (!/^[A-Za-z0-9_.]{2,16}$/.test(n)) return Promise.resolve(null);
  const key = n.toLowerCase();
  let hit = cache.get(key);
  if (!hit) {
    hit = new Promise((resolve) => {
      const url = `https://skinsystem.ely.by/skins/${encodeURIComponent(n)}.png`;
      const img = new Image();
      img.onload = () => resolve({ url, height: img.naturalHeight >= 64 ? 64 : 32 });
      img.onerror = () => {
        cache.delete(key); // allow a retry later (e.g. after coming online)
        resolve(null);
      };
      img.src = url;
    });
    cache.set(key, hit);
  }
  return hit;
}
