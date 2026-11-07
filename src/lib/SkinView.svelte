<script lang="ts">
  // Front view of a Minecraft skin (head, body, arms, legs and their overlay layers) drawn
  // straight from the skin texture with pixel-perfect scaling. With `stage` it sits on a lit
  // stage, floats gently, and turns toward the pointer; the overlay layers stand slightly
  // proud of the body so the turn has real depth.
  import { loadSkin, type Skin } from './skin';

  let { name, unit = 6, stage = false }: { name: string; unit?: number; stage?: boolean } = $props();

  let skin = $state<Skin | null>(null);
  let loading = $state(true);
  let ry = $state(0);
  let rx = $state(0);
  let resting = $state(true);

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

  function look(e: PointerEvent) {
    if (!stage) return;
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (e.clientX - box.left) / box.width - 0.5;
    const y = (e.clientY - box.top) / box.height - 0.5;
    ry = Math.max(-1, Math.min(1, x * 2)) * 32;
    rx = Math.max(-1, Math.min(1, y * 2)) * -9;
    resting = false;
  }
  function rest() {
    ry = 0;
    rx = 0;
    resting = true;
  }

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
    overlay?: boolean;
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
    const overlay: Part[] = [{ x: 4, y: 0, w: 8, h: 8, sx: 40, sy: 8, overlay: true }];
    if (modern) {
      overlay.push(
        { x: 4, y: 8, w: 8, h: 12, sx: 20, sy: 36, overlay: true },
        { x: 0, y: 8, w: 4, h: 12, sx: 44, sy: 36, overlay: true },
        { x: 12, y: 8, w: 4, h: 12, sx: 52, sy: 52, overlay: true },
        { x: 4, y: 20, w: 4, h: 12, sx: 4, sy: 36, overlay: true },
        { x: 8, y: 20, w: 4, h: 12, sx: 4, sy: 52, overlay: true },
      );
    }
    return [...base, ...overlay];
  });

  const place = (p: Part) =>
    `${p.overlay ? 'translateZ(calc(var(--u) * 0.7)) ' : ''}${p.flip ? 'scaleX(-1)' : ''}`.trim() || 'none';
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="scene" class:stage style:--u="{unit}px" onpointermove={look} onpointerleave={rest}>
  {#if stage}
    <span class="glow" aria-hidden="true"></span>
    <span class="floor" aria-hidden="true"></span>
  {/if}
  <div class="bob" class:float={stage && !!skin}>
    <div
      class="figure"
      class:empty={!skin}
      class:resting
      style:transform={stage ? `rotateY(${ry}deg) rotateX(${rx}deg)` : undefined}
      role="img"
      aria-label={skin ? `${name}'s skin` : 'No skin'}
    >
      {#if skin}
        {#each parts as p, i (i)}
          <span
            class="part"
            style:left="calc(var(--u) * {p.x})"
            style:top="calc(var(--u) * {p.y})"
            style:width="calc(var(--u) * {p.w})"
            style:height="calc(var(--u) * {p.h})"
            style:transform={place(p)}
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
  </div>
</div>

<style>
  .scene {
    position: relative;
    flex: none;
    width: calc(var(--u) * 16);
    height: calc(var(--u) * 32);
  }
  /* The stage grows the pointer area without moving anything around it */
  .scene.stage {
    box-sizing: content-box;
    padding: calc(var(--u) * 3) calc(var(--u) * 6);
    margin: calc(var(--u) * -3) calc(var(--u) * -6);
  }
  .bob {
    position: relative;
    width: calc(var(--u) * 16);
    height: calc(var(--u) * 32);
    perspective: calc(var(--u) * 90);
    filter: drop-shadow(0 10px 16px rgb(0 0 0 / 0.35));
  }
  .bob.float {
    animation: bob 4.6s ease-in-out infinite;
  }
  .figure {
    position: relative;
    width: 100%;
    height: 100%;
    transform-style: preserve-3d;
    transition: transform 420ms cubic-bezier(0.2, 0, 0, 1);
  }
  .figure:not(.resting) {
    transition: transform 90ms linear;
  }
  .figure.empty {
    filter: none;
  }
  .part {
    position: absolute;
    image-rendering: pixelated;
    background-repeat: no-repeat;
  }
  .part.ghost {
    background: color-mix(in srgb, var(--md-sys-color-on-surface, #ece0dd) 12%, transparent);
    outline: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface, #ece0dd) 6%, transparent);
  }
  .part.ghost.pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }

  .glow {
    position: absolute;
    left: 50%;
    top: 46%;
    width: calc(var(--u) * 30);
    height: calc(var(--u) * 30);
    transform: translate(-50%, -50%);
    border-radius: 50%;
    background: radial-gradient(
      closest-side,
      color-mix(in srgb, var(--md-sys-color-primary, #c23a24) 55%, transparent),
      transparent
    );
    filter: blur(calc(var(--u) * 1.6));
    opacity: 0.55;
    pointer-events: none;
  }
  .floor {
    position: absolute;
    left: 50%;
    bottom: calc(var(--u) * 1.4);
    width: calc(var(--u) * 15);
    height: calc(var(--u) * 2.4);
    transform: translateX(-50%);
    border-radius: 50%;
    background: radial-gradient(closest-side, rgb(0 0 0 / 0.5), transparent);
    pointer-events: none;
    animation: floor 4.6s ease-in-out infinite;
  }

  @keyframes bob {
    50% {
      transform: translateY(calc(var(--u) * -0.7));
    }
  }
  @keyframes floor {
    50% {
      transform: translateX(-50%) scale(0.88);
      opacity: 0.75;
    }
  }
  @keyframes pulse {
    50% {
      opacity: 0.5;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bob.float,
    .floor {
      animation: none;
    }
    .figure {
      transition: none;
    }
  }
</style>
