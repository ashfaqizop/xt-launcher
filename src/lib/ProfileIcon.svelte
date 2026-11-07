<script lang="ts">
  import Icon from './Icon.svelte';
  import LoaderLogo from './LoaderLogo.svelte';
  import { blockUri, onColor } from './blocks';
  import type { Profile } from './data.svelte';

  let {
    profile,
    size = 40,
  }: { profile: Pick<Profile, 'kind' | 'loader' | 'icon' | 'iconImage' | 'accent'>; size?: number } = $props();

  const accent = $derived(profile.accent || '');
  const picture = $derived(profile.icon === 'image' && profile.iconImage ? profile.iconImage : '');
  const block = $derived(!picture && profile.icon && profile.icon !== 'image' ? blockUri(profile.icon) : '');
  const isXt = $derived(profile.kind !== 'custom');
</script>

<span
  class="tile"
  class:xt={isXt}
  class:picture={!!picture}
  class:accented={!!accent}
  style:--size="{size}px"
  style:--accent={accent || undefined}
  style:--on={accent ? onColor(accent) : undefined}
  aria-hidden="true"
>
  {#if picture}
    <img src={picture} alt="" draggable="false" />
  {:else if block}
    <img class="block" src={block} alt="" draggable="false" />
  {:else if isXt}
    <Icon name="rocket" size={Math.round(size * 0.52)} />
  {:else}
    <LoaderLogo loader={profile.loader} size={Math.round(size * 0.56)} />
  {/if}
</span>

<style>
  .tile {
    width: var(--size);
    height: var(--size);
    flex: none;
    display: grid;
    place-items: center;
    border-radius: calc(var(--size) * 0.3);
    overflow: hidden;
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
    transition: background 200ms;
  }
  .tile.xt {
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .tile.accented {
    background: var(--accent);
    color: var(--on);
  }
  .tile.picture {
    background: transparent;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  img.block {
    width: 66%;
    height: 66%;
    image-rendering: pixelated;
    border-radius: 2px;
    filter: drop-shadow(0 2px 0 rgb(0 0 0 / 0.25));
  }
</style>
