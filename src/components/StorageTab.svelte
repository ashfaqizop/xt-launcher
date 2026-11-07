<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import ProfileIcon from '../lib/ProfileIcon.svelte';
  import { data } from '../lib/data.svelte';
  import { engine, inTauri } from '../lib/engine.svelte';
  import { showToast } from '../lib/toast.svelte';
  import { formatBytes } from '../lib/time';

  interface Entry {
    name: string;
    bytes: number;
    inUse: boolean;
  }
  interface Instance {
    id: string;
    bytes: number;
    saves: number;
    mods: number;
    screenshots: number;
    orphan: boolean;
  }
  interface Report {
    total: number;
    assets: number;
    libraries: number;
    cache: number;
    runtimes: Entry[];
    versions: Entry[];
    instances: Instance[];
    reclaimable: number;
    known: boolean;
  }

  let report = $state<Report | null>(null);
  let loading = $state(true);
  let error = $state('');
  let working = $state(false);
  let confirmClean = $state(false);
  let confirmOrphan = $state('');

  const ids = () => data.profiles.map((p) => p.id);

  async function load() {
    if (!inTauri) {
      loading = false;
      return;
    }
    loading = true;
    error = '';
    try {
      report = await invoke<Report>('storage_report', { profileIds: ids() });
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }
  onMount(load);

  const java = $derived(report ? report.runtimes.reduce((n, r) => n + r.bytes, 0) : 0);
  const versionBytes = $derived(report ? report.versions.reduce((n, r) => n + r.bytes, 0) : 0);
  const profileBytes = $derived(report ? report.instances.reduce((n, r) => n + r.bytes, 0) : 0);
  const unusedRuntimes = $derived(report ? report.runtimes.filter((r) => !r.inUse) : []);
  const unusedVersions = $derived(report ? report.versions.filter((r) => !r.inUse) : []);

  const segments = $derived.by(() => {
    if (!report) return [];
    const parts = [
      { id: 'profiles', label: 'Profiles', bytes: profileBytes, color: 'var(--md-sys-color-primary, #c23a24)' },
      { id: 'assets', label: 'Game assets', bytes: report.assets, color: 'var(--md-sys-color-tertiary, #e0c36a)' },
      { id: 'libraries', label: 'Libraries', bytes: report.libraries, color: '#3a8de0' },
      { id: 'java', label: 'Java', bytes: java, color: '#45b36b' },
      { id: 'versions', label: 'Versions', bytes: versionBytes, color: '#9a5ad6' },
      { id: 'cache', label: 'Cache', bytes: report.cache, color: 'var(--md-sys-color-outline, #9a8d89)' },
    ];
    return parts.filter((p) => p.bytes > 0);
  });
  const sum = $derived(segments.reduce((n, s) => n + s.bytes, 0) || 1);

  const profileOf = (id: string) => data.profiles.find((p) => p.id === id);
  const instances = $derived(report ? [...report.instances].sort((a, b) => b.bytes - a.bytes) : []);

  async function clean() {
    if (!confirmClean) {
      confirmClean = true;
      return;
    }
    confirmClean = false;
    working = true;
    try {
      const freed = await invoke<number>('storage_clean', { profileIds: ids(), what: ['runtimes', 'versions'] });
      showToast(`Freed ${formatBytes(freed)}`);
      await load();
    } catch (err) {
      showToast(String(err), 4800);
    } finally {
      working = false;
    }
  }

  async function clearCache() {
    working = true;
    try {
      const freed = await invoke<number>('storage_clean', { profileIds: ids(), what: ['cache'] });
      showToast(freed ? `Freed ${formatBytes(freed)}` : 'The cache was already empty');
      await load();
    } catch (err) {
      showToast(String(err));
    } finally {
      working = false;
    }
  }

  async function dropOrphan(id: string) {
    if (confirmOrphan !== id) {
      confirmOrphan = id;
      setTimeout(() => confirmOrphan === id && (confirmOrphan = ''), 4000);
      return;
    }
    confirmOrphan = '';
    working = true;
    try {
      const freed = await invoke<number>('storage_delete_instance', { id, keep: ids() });
      showToast(`Deleted the leftover data and freed ${formatBytes(freed)}`);
      await load();
    } catch (err) {
      showToast(String(err));
    } finally {
      working = false;
    }
  }

  const openRoot = () => invoke('open_dir', { kind: 'root', profileId: null }).catch((e) => showToast(String(e)));
  const busy = $derived(engine.phase !== 'idle');
</script>

<div class="storage">
  {#if loading && !report}
    <div class="ghost"><span class="skeleton"></span></div>
    <div class="ghost short"><span class="skeleton"></span></div>
  {:else if error}
    <div class="note bad"><Icon name="warning" size={22} /><span>{error}</span></div>
  {:else if report}
    <section class="summary">
      <div class="figure">
        <strong>{formatBytes(report.total)}</strong>
        <span class="muted">used by XT Launcher</span>
      </div>
      <m3e-button variant="text" size="small" onclick={openRoot}>
        <span slot="icon"><Icon name="folder" size={18} /></span>Open data folder
      </m3e-button>
      <m3e-icon-button aria-label="Measure again" disabled={loading} onclick={load}>
        <Icon name="refresh" size={20} />
      </m3e-icon-button>
    </section>

    <div class="bar" role="img" aria-label="Disk use by category">
      {#each segments as s (s.id)}
        <span class="seg" style:flex-grow={s.bytes / sum} style:background={s.color} title="{s.label}: {formatBytes(s.bytes)}"></span>
      {/each}
    </div>
    <ul class="legend">
      {#each segments as s (s.id)}
        <li><span class="dot" style:background={s.color}></span>{s.label}<b>{formatBytes(s.bytes)}</b></li>
      {/each}
    </ul>

    <section class="clean" class:ready={report.reclaimable > 0}>
      <span class="ci"><Icon name="clean" size={24} /></span>
      <div class="ct">
        {#if report.reclaimable > 0}
          <strong>{formatBytes(report.reclaimable)} can be freed</strong>
          <span class="muted">
            {unusedRuntimes.length
              ? `${unusedRuntimes.length} unused Java ${unusedRuntimes.length === 1 ? 'runtime' : 'runtimes'}`
              : ''}{unusedRuntimes.length && unusedVersions.length ? ' and ' : ''}{unusedVersions.length
              ? `${unusedVersions.length} game ${unusedVersions.length === 1 ? 'version' : 'versions'} no profile uses`
              : ''}. They download again if you ever need them.
          </span>
        {:else if !report.known && data.profiles.length}
          <strong>Play each profile once to unlock cleanup</strong>
          <span class="muted">XT then knows which Java runtimes and versions your profiles need, and what is safe to remove.</span>
        {:else}
          <strong>Nothing to clean up</strong>
          <span class="muted">Every Java runtime and game version is used by a profile.</span>
        {/if}
      </div>
      {#if report.reclaimable > 0}
        <m3e-button variant={confirmClean ? 'filled' : 'tonal'} disabled={working || busy} onclick={clean}>
          {confirmClean ? 'Yes, free it' : 'Clean up'}
        </m3e-button>
      {/if}
    </section>

    <h4>Profiles</h4>
    {#if !instances.length}
      <p class="muted small">No profile has any files yet.</p>
    {:else}
      <ul class="rows">
        {#each instances as inst (inst.id)}
          {@const p = profileOf(inst.id)}
          {@const other = Math.max(0, inst.bytes - inst.saves - inst.mods - inst.screenshots)}
          <li class="row">
            {#if p}
              <ProfileIcon profile={p} size={40} />
            {:else}
              <span class="ghosticon"><Icon name="warning" size={20} /></span>
            {/if}
            <div class="rt">
              <div class="rh">
                <strong>{p ? p.name : 'Leftover data'}</strong>
                {#if !p}<span class="tag warn">No profile</span>{/if}
              </div>
              <div class="mini" aria-hidden="true">
                <span style:flex-grow={inst.saves} class="m-saves"></span>
                <span style:flex-grow={inst.mods} class="m-mods"></span>
                <span style:flex-grow={inst.screenshots} class="m-shots"></span>
                <span style:flex-grow={other} class="m-other"></span>
              </div>
              <div class="muted small">
                {formatBytes(inst.saves)} worlds · {formatBytes(inst.mods)} mods · {formatBytes(inst.screenshots)} screenshots
              </div>
            </div>
            <span class="size">{formatBytes(inst.bytes)}</span>
            {#if inst.orphan}
              <m3e-button
                variant={confirmOrphan === inst.id ? 'filled' : 'text'}
                size="small"
                disabled={working}
                onclick={() => dropOrphan(inst.id)}
              >
                {confirmOrphan === inst.id ? 'Delete worlds too?' : 'Delete'}
              </m3e-button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    <h4>Java runtimes</h4>
    {#if !report.runtimes.length}
      <p class="muted small">None downloaded yet. Each Minecraft version gets the Java it needs when you first play it.</p>
    {:else}
      <ul class="rows compact">
        {#each report.runtimes as r (r.name)}
          <li class="row">
            <span class="ghosticon"><Icon name="hardDrive" size={20} /></span>
            <div class="rt"><strong>{r.name}</strong></div>
            <span class="size">{formatBytes(r.bytes)}</span>
            <span class="tag" class:warn={!r.inUse}>{r.inUse ? 'In use' : 'Unused'}</span>
          </li>
        {/each}
      </ul>
    {/if}

    <h4>Game versions and loaders</h4>
    {#if !report.versions.length}
      <p class="muted small">None installed yet.</p>
    {:else}
      <ul class="rows compact">
        {#each report.versions as v (v.name)}
          <li class="row">
            <span class="ghosticon"><Icon name="layers" size={20} /></span>
            <div class="rt"><strong class="mono">{v.name}</strong></div>
            <span class="size">{formatBytes(v.bytes)}</span>
            <span class="tag" class:warn={!v.inUse}>{v.inUse ? 'In use' : 'Unused'}</span>
          </li>
        {/each}
      </ul>
    {/if}

    {#if report.cache > 0}
      <div class="foot">
        <span class="muted small">Loader installers cached: {formatBytes(report.cache)}</span>
        <m3e-button variant="text" size="small" disabled={working || busy} onclick={clearCache}>Clear cache</m3e-button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .storage {
    padding: 4px 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
  .summary {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .figure {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .figure strong {
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }

  .bar {
    display: flex;
    gap: 3px;
    height: 16px;
    border-radius: 999px;
    overflow: hidden;
  }
  .seg {
    flex-basis: 0;
    min-width: 6px;
    border-radius: 4px;
    transition: flex-grow 500ms cubic-bezier(0.2, 0, 0, 1);
  }
  .seg:first-child {
    border-radius: 999px 4px 4px 999px;
  }
  .seg:last-child {
    border-radius: 4px 999px 999px 4px;
  }
  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    font-size: 13px;
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .legend b {
    color: var(--md-sys-color-on-surface, #ece0dd);
    font-variant-numeric: tabular-nums;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .clean {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px 14px 14px;
    border-radius: 22px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 50%, transparent);
  }
  .clean.ready {
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 55%, transparent);
  }
  .ci {
    width: 46px;
    height: 46px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 16px;
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .ct {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .ct .muted {
    font-size: 12.5px;
    line-height: 1.4;
  }
  .clean.ready .ct .muted {
    color: inherit;
    opacity: 0.8;
  }

  h4 {
    margin: 8px 0 -4px 4px;
    font-size: 12.5px;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px 10px 10px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 50%, transparent);
  }
  .compact .row {
    padding-block: 7px;
  }
  .ghosticon {
    width: 40px;
    height: 40px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 13px;
    background: var(--md-sys-color-surface-container-high, #2b2727);
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .compact .ghosticon {
    width: 34px;
    height: 34px;
    border-radius: 11px;
  }
  .rt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .rh {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rt strong {
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mono {
    font-variant-numeric: tabular-nums;
  }
  .size {
    flex: none;
    font-size: 13.5px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .tag {
    flex: none;
    font-size: 11px;
    font-weight: 650;
    padding: 2px 9px;
    border-radius: 999px;
    background: color-mix(in srgb, #45b36b 22%, transparent);
    color: #6fd495;
  }
  .tag.warn {
    background: color-mix(in srgb, var(--md-sys-color-error, #ffb4ab) 20%, transparent);
    color: var(--md-sys-color-error, #ffb4ab);
  }
  .mini {
    display: flex;
    gap: 2px;
    height: 6px;
    border-radius: 999px;
    overflow: hidden;
    background: color-mix(in srgb, var(--md-sys-color-on-surface, #fff) 6%, transparent);
  }
  .mini span {
    flex-basis: 0;
    min-width: 0;
  }
  .m-saves {
    background: #45b36b;
  }
  .m-mods {
    background: #9a5ad6;
  }
  .m-shots {
    background: #3a8de0;
  }
  .m-other {
    background: var(--md-sys-color-outline, #9a8d89);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 4px 0;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    border-radius: 18px;
  }
  .note.bad {
    color: var(--md-sys-color-error, #ffb4ab);
    background: color-mix(in srgb, var(--md-sys-color-error, #ffb4ab) 10%, transparent);
  }

  .ghost {
    height: 96px;
    border-radius: 22px;
    overflow: hidden;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 40%, transparent);
  }
  .ghost.short {
    height: 56px;
  }
  .skeleton {
    display: block;
    width: 100%;
    height: 100%;
    background: linear-gradient(
      100deg,
      transparent 20%,
      color-mix(in srgb, var(--md-sys-color-on-surface, #fff) 8%, transparent) 50%,
      transparent 80%
    );
    background-size: 220% 100%;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    to {
      background-position: -220% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton {
      animation: none;
    }
    .seg {
      transition: none;
    }
  }
</style>
