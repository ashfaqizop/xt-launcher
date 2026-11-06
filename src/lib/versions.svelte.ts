// Facts about Minecraft versions learned from the engine (kept separate so that
// both the data and engine modules can read them without importing each other).
export interface VersionItem {
  id: string;
  kind: string;
}

export const versions = $state({ latestRelease: '', list: [] as VersionItem[] });
