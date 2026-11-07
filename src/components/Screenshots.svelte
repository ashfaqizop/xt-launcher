<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Icon from '../lib/Icon.svelte';
  import { lazy, portal } from '../lib/actions';
  import { engine, inTauri } from '../lib/engine.svelte';
  import {
    copyScreenshot,
    deleteScreenshot,
    fullImage,
    listScreenshots,
    openScreenshot,
    thumbnail,
    type Screenshot,
  } from '../lib/screenshots';
  import { showToast } from '../lib/toast.svelte';
  import { ago } from '../lib/time';

  let {
    profileId,
    limit,
    layout = 'grid',
    onSeeAll,
  }: { profileId: string; limit?: number; layout?: 'strip' | 'grid'; onSeeAll?: () => void } = $props();

  let shots = $state<Screenshot[]>([]);
  let total = $state(0);
  let loaded = $state(false);
  let urls = $state<Record<string, string>>({});
  let viewing = $state(-1);
  let bigUrl = $state('');

  async function load() {
    if (!inTauri || !profileId) return;
    try {
      const all = await listScreenshots(profileId);
      total = all.length;
      shots = limit ? all.slice(0, limit) : all;
    } catch {
      shots = [];
      total = 0;
    }
    loaded = true;
  }

  // Reload for a different profile, and again when a game session ends (new screenshots)
  $effect(() => {
    void profileId;
    loaded = false;
    void load();
  });
  let wasBusy = false;
  $effect(() => {
    const busy = engine.phase !== 'idle';
    if (wasBusy && !busy) void load();
    wasBusy = busy;
  });

  async function ensure(shot: Screenshot) {
    if (urls[shot.name]) return;
    try {
      urls[shot.name] = await thumbnail(profileId, shot);
    } catch {
      /* leave the placeholder */
    }
  }

  // ---- actions -----------------------------------------------------------------

  async function open(shot: Screenshot) {
    try {
      await openScreenshot(profileId, shot.name);
    } catch (err) {
      showToast(String(err));
    }
  }
  async function copy(shot: Screenshot) {
    try {
      await copyScreenshot(profileId, shot.name);
      showToast('Screenshot copied to the clipboard');
    } catch {
      showToast('Could not copy the screenshot');
    }
  }
  async function remove(shot: Screenshot) {
    try {
      await deleteScreenshot(profileId, shot.name);
      const was = viewing;
      shots = shots.filter((s) => s.name !== shot.name);
      total = Math.max(0, total - 1);
      if (was >= 0) viewing = shots.length ? Math.min(was, shots.length - 1) : -1;
      showToast('Moved to the Recycle Bin');
    } catch (err) {
      showToast(String(err));
    }
  }
  const openFolder = () => invoke('open_dir', { kind: 'screenshots', profileId }).catch((e) => showToast(String(e)));

  // ---- lightbox ----------------------------------------------------------------

  const current = $derived(viewing >= 0 ? shots[viewing] : undefined);

  $effect(() => {
    const shot = current;
    bigUrl = '';
    if (!shot) return;
    let alive = true;
    let made = '';
    fullImage(profileId, shot.name)
      .then((url) => {
        if (alive) {
          made = url;
          bigUrl = url;
        } else URL.revokeObjectURL(url);
      })
      .catch(() => {});
    return () => {
      alive = false;
      if (made) URL.revokeObjectURL(made);
    };
  });

  function step(delta: number) {
    if (!shots.length) return;
    viewing = (viewing + delta + shots.length) % shots.length;
  }
  function key(e: KeyboardEvent) {
    if (viewing < 0) return;
    if (e.key === 'Escape') viewing = -1;
    else if (e.key === 'ArrowRight') step(1);
    else if (e.key === 'ArrowLeft') step(-1);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window onkeydowncapture={key} />

<section class="shots {layout}">
  <header>
    <h3>Screenshots{total ? ` · ${total}` : ''}</h3>
    <span class="grow"></span>
    {#if layout === 'strip' && onSeeAll && total > shots.length}
      <m3e-button variant="text" size="small" onclick={onSeeAll}>See all</m3e-button>
    {/if}
    <m3e-button variant="tonal" size="small" onclick={openFolder}>
      <span slot="icon"><Icon name="folder" size={18} /></span>Open folder
    </m3e-button>
  </header>

  {#if !loaded}
    <ul class="grid">
      {#each Array(layout === 'strip' ? 4 : 6) as _, i (i)}
        <li class="tile"><span class="skeleton"></span></li>
      {/each}
    </ul>
  {:else if !shots.length}
    <div class="empty">
      <Icon name="image" size={30} />
      <p>No screenshots yet</p>
      <span class="muted">Press F2 in game and they show up here.</span>
    </div>
  {:else}
    <ul class="grid">
      {#each shots as s, i (s.name)}
        <li class="tile" use:lazy={() => ensure(s)}>
          <button class="pic" aria-label="View the screenshot from {ago(s.modified)}" onclick={() => (viewing = i)}>
            {#if urls[s.name]}
              <img src={urls[s.name]} alt="" draggable="false" />
            {:else}
              <span class="skeleton"></span>
            {/if}
          </button>
          <span class="when">{ago(s.modified)}</span>
          <div class="acts">
            <button type="button" title="Open in your image viewer" aria-label="Open" onclick={() => open(s)}><Icon name="open" size={18} /></button>
            <button type="button" title="Copy to the clipboard" aria-label="Copy" onclick={() => copy(s)}><Icon name="copy" size={18} /></button>
            <button type="button" class="danger" title="Move to the Recycle Bin" aria-label="Delete" onclick={() => remove(s)}><Icon name="trash" size={18} /></button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if current}
  <div class="lightbox" use:portal role="dialog" aria-modal="true" aria-label="Screenshot">
    <button class="scrim" aria-label="Close" onclick={() => (viewing = -1)}></button>
    <div class="stage">
      {#if bigUrl}
        <img src={bigUrl} alt="Screenshot from {ago(current.modified)}" />
      {:else}
        <m3e-circular-progress-indicator variant="wavy" mode="indeterminate"></m3e-circular-progress-indicator>
      {/if}
    </div>
    {#if shots.length > 1}
      <button class="nav prev" aria-label="Previous" onclick={() => step(-1)}><Icon name="chevronLeft" size={28} /></button>
      <button class="nav next" aria-label="Next" onclick={() => step(1)}><Icon name="chevronRight" size={28} /></button>
    {/if}
    <div class="bar">
      <div class="cap">
        <strong>{ago(current.modified)}</strong>
        <span>{current.name} · {viewing + 1} of {shots.length}</span>
      </div>
      <button type="button" aria-label="Open" title="Open in your image viewer" onclick={() => open(current)}><Icon name="open" size={20} /></button>
      <button type="button" aria-label="Copy" title="Copy to the clipboard" onclick={() => copy(current)}><Icon name="copy" size={20} /></button>
      <button type="button" class="danger" aria-label="Delete" title="Move to the Recycle Bin" onclick={() => remove(current)}><Icon name="trash" size={20} /></button>
      <button type="button" aria-label="Close" title="Close (Esc)" onclick={() => (viewing = -1)}><Icon name="close" size={20} /></button>
    </div>
  </div>
{/if}

<style>
  .shots {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }
  .grow {
    flex: 1;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 10px;
  }
  .strip .grid {
    display: flex;
    overflow-x: auto;
    padding-bottom: 8px;
    scroll-snap-type: x proximity;
  }
  .strip .tile {
    flex: 0 0 200px;
    scroll-snap-align: start;
  }
  .tile {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: 16px;
    overflow: hidden;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 70%, transparent);
    animation: pop 260ms cubic-bezier(0.2, 0, 0, 1) both;
  }
  .pic {
    all: unset;
    display: block;
    width: 100%;
    height: 100%;
    cursor: zoom-in;
  }
  .pic:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    outline-offset: -2px;
    border-radius: 16px;
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 320ms cubic-bezier(0.2, 0, 0, 1);
  }
  .tile:hover .pic img {
    transform: scale(1.04);
  }
  .skeleton {
    display: block;
    width: 100%;
    height: 100%;
    background: linear-gradient(
      100deg,
      transparent 20%,
      color-mix(in srgb, var(--md-sys-color-on-surface, #fff) 9%, transparent) 50%,
      transparent 80%
    );
    background-size: 220% 100%;
    animation: shimmer 1.4s linear infinite;
  }

  .when {
    position: absolute;
    left: 8px;
    top: 8px;
    padding: 3px 9px;
    border-radius: 999px;
    font-size: 11.5px;
    font-weight: 600;
    color: #fff;
    background: rgb(0 0 0 / 0.5);
    -webkit-backdrop-filter: blur(8px);
    backdrop-filter: blur(8px);
    pointer-events: none;
    opacity: 0;
    transition: opacity 140ms;
  }
  .acts {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    padding: 22px 8px 8px;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.62));
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 140ms,
      transform 140ms;
    pointer-events: none;
  }
  .tile:hover .when,
  .tile:focus-within .when,
  .tile:hover .acts,
  .tile:focus-within .acts {
    opacity: 1;
  }
  .tile:hover .acts,
  .tile:focus-within .acts {
    transform: none;
    pointer-events: auto;
  }
  .acts button,
  .bar button {
    all: unset;
    box-sizing: border-box;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    flex: none;
    border-radius: 50%;
    color: #fff;
    cursor: pointer;
    background: rgb(255 255 255 / 0.16);
    transition: background 120ms;
  }
  .acts button:hover,
  .bar button:hover {
    background: rgb(255 255 255 / 0.3);
  }
  .acts button.danger:hover,
  .bar button.danger:hover {
    background: #c0392b;
  }
  .acts button:focus-visible,
  .bar button:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  .empty {
    display: grid;
    justify-items: center;
    gap: 2px;
    padding: 26px 12px;
    border-radius: 20px;
    text-align: center;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 40%, transparent);
  }
  .empty p {
    margin: 6px 0 0;
    font-weight: 600;
    color: var(--md-sys-color-on-surface, #ece0dd);
  }

  /* Lightbox (moved to the themed root by the portal action) */
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    animation: fade 180ms ease-out;
  }
  .scrim {
    all: unset;
    position: absolute;
    inset: 0;
    background: rgb(0 0 0 / 0.82);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
    cursor: zoom-out;
  }
  .stage {
    position: relative;
    max-width: calc(100vw - 150px);
    max-height: calc(100vh - 150px);
    display: grid;
    place-items: center;
    pointer-events: none;
  }
  .stage img {
    max-width: calc(100vw - 150px);
    max-height: calc(100vh - 150px);
    border-radius: 14px;
    box-shadow: 0 24px 80px rgb(0 0 0 / 0.6);
    animation: zoom 240ms cubic-bezier(0.2, 0, 0, 1);
  }
  .nav {
    all: unset;
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: #fff;
    cursor: pointer;
    background: rgb(255 255 255 / 0.14);
  }
  .nav:hover {
    background: rgb(255 255 255 / 0.28);
  }
  .nav.prev {
    left: 24px;
  }
  .nav.next {
    right: 24px;
  }
  .nav:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }
  .bar {
    position: absolute;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px 8px 18px;
    border-radius: 999px;
    color: #fff;
    background: rgb(20 20 20 / 0.7);
    -webkit-backdrop-filter: blur(16px);
    backdrop-filter: blur(16px);
  }
  .cap {
    display: flex;
    flex-direction: column;
    margin-right: 10px;
    font-size: 12px;
    line-height: 1.3;
    max-width: 280px;
  }
  .cap strong {
    font-size: 14px;
  }
  .cap span {
    opacity: 0.7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
  @keyframes shimmer {
    to {
      background-position: -220% 0;
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes zoom {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tile,
    .skeleton,
    .stage img,
    .lightbox {
      animation: none;
    }
  }
</style>
