<script lang="ts">
  // A player's face (with hat layer) cut out of their Ely.by skin; the first letter
  // of the name is shown when there is no skin.
  import { loadSkin } from './skin';

  let { name, size = 40 }: { name: string; size?: number } = $props();

  let skin = $state<string | null>(null);

  $effect(() => {
    const n = name;
    let alive = true;
    skin = null;
    void loadSkin(n).then((s) => alive && (skin = s?.url ?? null));
    return () => {
      alive = false;
    };
  });
</script>

<span
  class="head"
  class:skin={!!skin}
  style:--s="{size}px"
  style:background-image={skin ? `url("${skin}"), url("${skin}")` : undefined}
  aria-hidden="true"
>
  {#if !skin}{name.trim().slice(0, 1).toUpperCase() || '?'}{/if}
</span>

<style>
  .head {
    width: var(--s);
    height: var(--s);
    flex: none;
    display: grid;
    place-items: center;
    border-radius: calc(var(--s) * 0.28);
    overflow: hidden;
    font-weight: 700;
    font-size: calc(var(--s) * 0.42);
    background-color: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  /* Face is the 8x8 square at (8,8) of a 64-wide skin; the hat layer sits at (40,8). */
  .head.skin {
    image-rendering: pixelated;
    background-repeat: no-repeat;
    background-size:
      calc(var(--s) * 8) auto,
      calc(var(--s) * 8) auto;
    background-position:
      calc(var(--s) * -5) calc(var(--s) * -1),
      calc(var(--s) * -1) calc(var(--s) * -1);
  }
</style>
