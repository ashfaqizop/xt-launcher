<script lang="ts">
  import { onMount } from 'svelte';
  import { memory, refreshMemory } from '../lib/memory.svelte';
  import { gib, maxGameMemoryGiB } from '../lib/system.svelte';

  let {
    value,
    onchange,
    label = 'Memory in GiB',
  }: { value: number; onchange: (gib: number) => void; label?: string } = $props();

  const uid = $props.id();
  const id = `ram-${uid}`;

  const max = $derived(maxGameMemoryGiB());
  const shown = $derived(Math.min(value, max));
  const freeText = $derived(memory.freeMb ? `${gib(memory.freeMb)} GiB` : '');
  const over = $derived(memory.freeMb > 0 && shown * 1024 > memory.freeMb);

  onMount(() => {
    void refreshMemory();
    const timer = setInterval(() => {
      if (document.visibilityState === 'visible') void refreshMemory();
    }, 4000);
    return () => clearInterval(timer);
  });
</script>

<div class="ram" onpointerenter={refreshMemory} onfocusin={refreshMemory}>
  <div class="row">
    <m3e-slider {id} min="1" {max} step="0.5" labelled class="slider" aria-label={label}>
      <m3e-slider-thumb value={shown} oninput={(e: Event) => onchange(Number((e.currentTarget as HTMLInputElement).value))}></m3e-slider-thumb>
    </m3e-slider>
    <span class="amount">{gib(shown * 1024)} GiB</span>
  </div>
  {#if freeText}
    <p class="hint" class:warn={over}>
      <span class="dot"></span>
      {over ? `Only ${freeText} is free right now. Close other apps or pick less.` : `${freeText} free right now`}
    </p>
  {/if}
  <m3e-tooltip for={id}>
    {#if memory.totalMb}
      Free right now: {freeText} of {gib(memory.totalMb)} GiB
    {:else}
      Checking memory…
    {/if}
  </m3e-tooltip>
</div>

<style>
  .ram {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .slider {
    flex: 1;
  }
  .amount {
    min-width: 62px;
    text-align: right;
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .hint {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #45b36b;
    flex: none;
  }
  .hint.warn {
    color: var(--md-sys-color-error, #ffb4ab);
  }
  .hint.warn .dot {
    background: var(--md-sys-color-error, #ffb4ab);
  }
</style>
