// First-run and after-update moments: "What's new" with the release notes, and the
// low-memory suggestion for small PCs.

import changelog from '../../CHANGELOG.md?raw';
import { data, profileFromTemplate, saveProfile } from './data.svelte';
import { settings } from './settings.svelte';
import { lowEndMemoryMb, loadSystem, system } from './system.svelte';
import { compareVersions } from './time';

export interface ReleaseNotes {
  version: string;
  groups: { title: string; items: string[] }[];
}

/** Parse CHANGELOG.md: "## 0.1.3", "### New", "- item". Newest first, as written. */
export function parseChangelog(text: string): ReleaseNotes[] {
  const releases: ReleaseNotes[] = [];
  let release: ReleaseNotes | null = null;
  let group: { title: string; items: string[] } | null = null;

  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trimEnd();
    const version = /^##\s+v?(\d+(?:\.\d+)*)/.exec(line);
    if (version) {
      release = { version: version[1], groups: [] };
      releases.push(release);
      group = null;
      continue;
    }
    const title = /^###\s+(.+)/.exec(line);
    if (title && release) {
      group = { title: title[1], items: [] };
      release.groups.push(group);
      continue;
    }
    const item = /^[-*]\s+(.+)/.exec(line);
    if (item && release) {
      if (!group) {
        group = { title: 'Changes', items: [] };
        release.groups.push(group);
      }
      group.items.push(item[1]);
    }
  }
  return releases;
}

export const welcome = $state({
  /** Releases to show in "What's new" (null = nothing to show) */
  notes: null as ReleaseNotes[] | null,
  /** Offer XT Optimized with lower memory (a small PC on first run) */
  lowEnd: false,
  version: '',
});

/** Decide what, if anything, to show now that the main window is up. */
export async function initWelcome(currentVersion: string) {
  welcome.version = currentVersion;
  const hasData = data.profiles.length > 0 || data.accounts.length > 0;

  if (!settings.seenVersion) {
    // Before 0.1.3 nothing was recorded: anyone with data is upgrading from 0.1.2
    settings.seenVersion = hasData ? '0.1.2' : currentVersion;
  }

  if (compareVersions(currentVersion, settings.seenVersion) > 0) {
    const seen = settings.seenVersion;
    const notes = parseChangelog(changelog)
      .filter((r) => compareVersions(r.version, seen) > 0 && compareVersions(r.version, currentVersion) <= 0)
      .slice(0, 3);
    if (notes.length) welcome.notes = notes;
    else settings.seenVersion = currentVersion;
  }

  // First run on a small PC
  if (!settings.lowEndChecked) {
    if (hasData) {
      settings.lowEndChecked = true;
    } else {
      await loadSystem();
      if (system.totalMemoryMb > 0) {
        if (system.totalMemoryMb <= 8.5 * 1024) welcome.lowEnd = true;
        else settings.lowEndChecked = true;
      }
    }
  }
}

export function dismissWhatsNew() {
  welcome.notes = null;
  if (welcome.version) settings.seenVersion = welcome.version;
}

export function dismissLowEnd() {
  welcome.lowEnd = false;
  settings.lowEndChecked = true;
}

/** Create XT Optimized with a memory limit that suits this PC and make it current. */
export function createLowEndProfile() {
  const profile = profileFromTemplate('xt');
  profile.ramMb = lowEndMemoryMb();
  saveProfile(profile);
  data.profileId = profile.id;
  dismissLowEnd();
}
