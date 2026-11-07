<script lang="ts">
  import { blockUri } from './blocks';
  import type { Loader } from './data.svelte';
  import { LOADER_SVG } from './loaders';

  let { loader, size = 24 }: { loader: Loader; size?: number } = $props();
</script>

{#if loader === 'vanilla'}
  <img class="logo block" src={blockUri('grass')} alt="" style:--s="{size}px" draggable="false" />
{:else}
  <span class="logo" style:--s="{size}px" aria-hidden="true">{@html LOADER_SVG[loader]}</span>
{/if}

<style>
  .logo {
    display: inline-grid;
    place-items: center;
    width: var(--s);
    height: var(--s);
    flex: none;
    line-height: 0;
  }
  .logo :global(svg) {
    width: 100%;
    height: 100%;
  }
  .logo.block {
    image-rendering: pixelated;
    border-radius: 2px;
  }
</style>
