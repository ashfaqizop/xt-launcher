<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { selectValue } from '../lib/actions';
  import { LOADER_LABEL, type Profile } from '../lib/data.svelte';
  import { inTauri } from '../lib/engine.svelte';
  import { installedProjectIds, loadMods, modsState } from '../lib/mods.svelte';
  import { showToast } from '../lib/toast.svelte';
  import { compact } from '../lib/time';
  import { versions } from '../lib/versions.svelte';

  let { profile, onback }: { profile: Profile; onback: () => void } = $props();

  interface Hit {
    projectId: string;
    slug: string;
    title: string;
    description: string;
    iconUrl: string | null;
    downloads: number;
    follows: number;
    author: string;
    categories: string[];
  }
  interface Page {
    hits: Hit[];
    total: number;
    mc: string;
  }
  interface Report {
    installed: { title: string; file: string; dependency: boolean }[];
    skipped: string[];
  }

  const SORTS = [
    { id: 'downloads', label: 'Most downloaded' },
    { id: 'relevance', label: 'Best match' },
    { id: 'follows', label: 'Most followed' },
    { id: 'updated', label: 'Recently updated' },
    { id: 'newest', label: 'Newest' },
  ];

  let query = $state('');
  let sort = $state('downloads');
  let hits = $state<Hit[]>([]);
  let total = $state(0);
  let mc = $state('');
  let loading = $state(true);
  let more = $state(false);
  let error = $state('');
  let installing = $state<Record<string, boolean>>({});

  const installed = $derived(installedProjectIds());
  const mcLabel = $derived(mc || (profile.mc === 'latest' ? versions.latestRelease : profile.mc));

  let token = 0;
  async function run(offset: number) {
    if (!inTauri) {
      loading = false;
      return;
    }
    const mine = ++token;
    if (offset === 0) loading = true;
    else more = true;
    error = '';
    try {
      const page = await invoke<Page>('modrinth_search', {
        query,
        mc: profile.mc,
        loader: profile.loader,
        sort,
        offset,
      });
      if (mine !== token) return;
      hits = offset === 0 ? page.hits : [...hits, ...page.hits];
      total = page.total;
      mc = page.mc;
    } catch (err) {
      if (mine !== token) return;
      error = String(err);
      if (offset === 0) hits = [];
    } finally {
      if (mine === token) {
        loading = false;
        more = false;
      }
    }
  }

  // Search as you type (debounced); a sort change searches right away
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void query;
    void sort;
    clearTimeout(timer);
    timer = setTimeout(() => void run(0), query ? 350 : 0);
    return () => clearTimeout(timer);
  });

  onMount(() => {
    if (modsState.profileId !== profile.id) void loadMods(profile.id, true);
  });

  async function install(hit: Hit) {
    installing[hit.projectId] = true;
    try {
      const report = await invoke<Report>('modrinth_install', {
        profileId: profile.id,
        project: hit.projectId,
        mc: profile.mc,
        loader: profile.loader,
      });
      const extra = report.installed.filter((i) => i.dependency).length;
      showToast(
        extra
          ? `Installed ${hit.title} and ${extra} required ${extra === 1 ? 'mod' : 'mods'}`
          : `Installed ${hit.title}`,
        3600,
      );
      if (report.skipped.length) showToast(report.skipped[0], 5200);
      await loadMods(profile.id, true);
    } catch (err) {
      showToast(String(err), 5200);
    } finally {
      installing[hit.projectId] = false;
    }
  }

  const val = (e: Event) => (e.currentTarget as HTMLElement & { value: string }).value;
  const hasMore = $derived(hits.length < total);
</script>

<div class="browser">
  <header>
    <m3e-icon-button aria-label="Back to this profile's mods" onclick={onback}>
      <Icon name="back" />
    </m3e-icon-button>
    <div class="heading">
      <h3>Browse Modrinth</h3>
      <span class="muted">
        Showing mods for {LOADER_LABEL[profile.loader]}{mcLabel ? ` on Minecraft ${mcLabel}` : ''}
      </span>
    </div>
  </header>

  <div class="tools">
    <m3e-form-field variant="outlined" class="search">
      <span slot="prefix" class="pre"><Icon name="search" size={20} /></span>
      <label slot="label" for="mr-q">Search mods</label>
      <input id="mr-q" bind:value={query} autocomplete="off" spellcheck="false" />
    </m3e-form-field>
    <m3e-form-field variant="outlined" class="sort">
      <label slot="label" for="mr-sort">Sort by</label>
      <m3e-select id="mr-sort" use:selectValue={sort} onchange={(e: Event) => (sort = val(e))}>
        {#each SORTS as s (s.id)}
          <m3e-option value={s.id}>{s.label}</m3e-option>
        {/each}
      </m3e-select>
    </m3e-form-field>
  </div>

  {#if loading}
    <ul class="results">
      {#each Array(5) as _, i (i)}
        <li class="card ghost"><span class="skeleton"></span></li>
      {/each}
    </ul>
  {:else if error}
    <div class="notice bad">
      <Icon name="warning" size={22} />
      <div>
        <strong>Couldn't search Modrinth</strong>
        <span>{error}</span>
      </div>
      <m3e-button variant="tonal" size="small" onclick={() => run(0)}>Try again</m3e-button>
    </div>
  {:else if !hits.length}
    <div class="notice">
      <Icon name="search" size={22} />
      <div>
        <strong>No mods found</strong>
        <span>Nothing for {LOADER_LABEL[profile.loader]} on Minecraft {mcLabel} matches "{query}".</span>
      </div>
    </div>
  {:else}
    <ul class="results">
      {#each hits as h (h.projectId)}
        {@const isIn = installed.has(h.projectId)}
        <li class="card">
          {#if h.iconUrl}
            <img class="icon" src={h.iconUrl} alt="" loading="lazy" draggable="false" />
          {:else}
            <span class="icon letter">{h.title.slice(0, 1).toUpperCase()}</span>
          {/if}
          <div class="body">
            <div class="title">
              <strong>{h.title}</strong>
              <span class="muted">by {h.author}</span>
            </div>
            <p class="desc">{h.description}</p>
            <div class="meta">
              <span title="Downloads"><Icon name="download" size={14} />{compact(h.downloads)}</span>
              <span title="Followers">{compact(h.follows)} followers</span>
              {#each h.categories.slice(0, 3) as c (c)}
                <span class="chip">{c}</span>
              {/each}
            </div>
          </div>
          <div class="cta">
            {#if isIn}
              <span class="done"><Icon name="checkCircle" size={18} />Installed</span>
            {:else}
              <m3e-button variant="tonal" size="small" disabled={installing[h.projectId]} onclick={() => install(h)}>
                {#if installing[h.projectId]}
                  <m3e-circular-progress-indicator slot="icon" variant="wavy" mode="indeterminate" class="spin"></m3e-circular-progress-indicator>
                  Installing…
                {:else}
                  <span slot="icon"><Icon name="download" size={18} /></span>Install
                {/if}
              </m3e-button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if hasMore}
      <div class="more">
        <m3e-button variant="text" disabled={more} onclick={() => run(hits.length)}>
          {more ? 'Loading…' : `Show more (${total - hits.length} left)`}
        </m3e-button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .browser {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .heading {
    display: flex;
    flex-direction: column;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }
  .heading .muted {
    font-size: 12.5px;
  }
  .tools {
    display: grid;
    grid-template-columns: 1fr 200px;
    gap: 10px;
  }
  .pre {
    display: grid;
    place-items: center;
    padding-left: 4px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }

  .results {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px 12px 12px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
    animation: rise 260ms cubic-bezier(0.2, 0, 0, 1) both;
  }
  .card.ghost {
    height: 88px;
    padding: 0;
    overflow: hidden;
  }
  .skeleton {
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
  .icon {
    width: 56px;
    height: 56px;
    flex: none;
    border-radius: 16px;
    object-fit: cover;
    background: var(--md-sys-color-surface-container-high, #2b2727);
  }
  .icon.letter {
    display: grid;
    place-items: center;
    font-size: 22px;
    font-weight: 700;
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
    background: var(--md-sys-color-secondary-container, #5d3f3a);
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .title strong {
    font-size: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title .muted {
    font-size: 12.5px;
    white-space: nowrap;
  }
  .desc {
    margin: 0;
    font-size: 13px;
    line-height: 1.4;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 12px;
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .meta span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .chip {
    padding: 1px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface, #fff) 8%, transparent);
    text-transform: capitalize;
  }
  .cta {
    flex: none;
    min-width: 108px;
    display: flex;
    justify-content: flex-end;
  }
  .done {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: #45b36b;
  }
  .spin {
    --m3e-progress-indicator-size: 18px;
    width: 18px;
    height: 18px;
  }
  .more {
    display: flex;
    justify-content: center;
    padding: 4px 0 2px;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 18px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 45%, transparent);
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .notice div {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 2px;
    font-size: 13px;
  }
  .notice strong {
    color: var(--md-sys-color-on-surface, #ece0dd);
    font-size: 14px;
  }
  .notice.bad {
    color: var(--md-sys-color-error, #ffb4ab);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  @keyframes shimmer {
    to {
      background-position: -220% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card,
    .skeleton {
      animation: none;
    }
  }
</style>
