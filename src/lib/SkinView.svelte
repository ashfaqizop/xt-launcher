<script lang="ts">
  // Flat front view of a Minecraft skin (head, body, arms, legs and their overlay
  // layers), drawn straight from the skin texture with pixel-perfect scaling.
  import { loadSkin, type Skin } from './skin';

  let { name, unit = 6 }: { name: string; unit?: number } = $props();

  let skin = $state<Skin | null>(null);
  let loading = $state(true);

  $effect(() => {
    const n = name;
    let alive = true;
    loading = true;
    void loadSkin(n).then((s) => {
      if (!alive) return;
      skin = s;
      loading = false;
    });
    return () => {
      alive = false;
    };
  });

  interface Part {
    /** position in the 16x32 figure */
    x: number;
    y: number;
    w: number;
    h: number;
    /** source position in the skin texture */
    sx: number;
    sy: number;
    flip?: boolean;
  }

  const SILHOUETTE = [
    { x: 4, y: 0, w: 8, h: 8 },
    { x: 4, y: 8, w: 8, h: 12 },
    { x: 0, y: 8, w: 4, h: 12 },
    { x: 12, y: 8, w: 4, h: 12 },
    { x: 4, y: 20, w: 4, h: 12 },
    { x: 8, y: 20, w: 4, h: 12 },
  ];

  // Base layer, then overlays. Legacy 64x32 skins mirror the right limbs.
  const parts = $derived.by((): Part[] => {
    const modern = (skin?.height ?? 64) >= 64;
    const base: Part[] = [
      { x: 4, y: 0, w: 8, h: 8, sx: 8, sy: 8 }, // head
      { x: 4, y: 8, w: 8, h: 12, sx: 20, sy: 20 }, // body
      { x: 0, y: 8, w: 4, h: 12, sx: 44, sy: 20 }, // right arm
      modern ? { x: 12, y: 8, w: 4, h: 12, sx: 36, sy: 52 } : { x: 12, y: 8, w: 4, h: 12, sx: 44, sy: 20, flip: true },
      { x: 4, y: 20, w: 4, h: 12, sx: 4, sy: 20 }, // right leg
      modern ? { x: 8, y: 20, w: 4, h: 12, sx: 20, sy: 52 } : { x: 8, y: 20, w: 4, h: 12, sx: 4, sy: 20, flip: true },
    ];
    const overlay: Part[] = [{ x: 4, y: 0, w: 8, h: 8, sx: 40, sy: 8 }];
    if (modern) {
      overlay.push(
        { x: 4, y: 8, w: 8, h: 12, sx: 20, sy: 36 },
        { x: 0, y: 8, w: 4, h: 12, sx: 44, sy: 36 },
        { x: 12, y: 8, w: 4, h: 12, sx: 52, sy: 52 },
        { x: 4, y: 20, w: 4, h: 12, sx: 4, sy: 36 },
        { x: 8, y: 20, w: 4, h: 12, sx: 4, sy: 52 },
      );
    }
    return [...base, ...overlay];
  });
</script>

<div class="figure" style:--u="{unit}px" class:empty={!skin} aria-label={skin ? `${name}'s skin` : 'No skin'}>
  {#if skin}
    {#each parts as p, i (i)}
      <span
        class="part"
        class:flip={p.flip}
        style:left="calc(var(--u) * {p.x})"
        style:top="calc(var(--u) * {p.y})"
        style:width="calc(var(--u) * {p.w})"
        style:height="calc(var(--u) * {p.h})"
        style:background-image={`url("${skin.url}")`}
        style:background-size="calc(var(--u) * 64) auto"
        style:background-position="calc(var(--u) * {-p.sx}) calc(var(--u) * {-p.sy})"
      ></span>
    {/each}
  {:else}
    <!-- No skin: a neutral silhouette in the same proportions -->
    {#each SILHOUETTE as p, i (i)}
      <span
        class="part ghost"
        class:pulse={loading}
        style:left="calc(var(--u) * {p.x})"
        style:top="calc(var(--u) * {p.y})"
        style:width="calc(var(--u) * {p.w})"
        style:height="calc(var(--u) * {p.h})"
      ></span>
    {/each}
  {/if}
</div>

<style>
  .figure {
    position: relative;
    width: calc(var(--u) * 16);
    height: calc(var(--u) * 32);
    flex: none;
    filter: drop-shadow(0 10px 18px rgb(0 0 0 / 0.35));
  }
  .part {
    position: absolute;
    image-rendering: pixelated;
    background-repeat: no-repeat;
  }
  .part.flip {
    transform: scaleX(-1);
  }
  .figure.empty {
    filter: none;
  }
  .part.ghost {
    background: color-mix(in srgb, var(--md-sys-color-on-surface, #ece0dd) 12%, transparent);
    outline: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface, #ece0dd) 6%, transparent);
  }
  .part.ghost.pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.5;
    }
  }
</style>
