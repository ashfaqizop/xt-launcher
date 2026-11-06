<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import { engine, stop } from '../lib/engine.svelte';
  import { showToast } from '../lib/toast.svelte';

  const SHOWN = 1500;
  const lines = $derived(engine.logs.length > SHOWN ? engine.logs.slice(-SHOWN) : engine.logs);

  let box: HTMLDivElement;
  let stick = true;

  function onScroll() {
    stick = box.scrollHeight - box.scrollTop - box.clientHeight < 40;
  }

  $effect(() => {
    void lines.length;
    if (stick) void tick().then(() => box && (box.scrollTop = box.scrollHeight));
  });

  async function copy() {
    try {
      await navigator.clipboard.writeText(engine.logs.map((l) => l.line).join('\n'));
      showToast('Log copied');
    } catch {
      showToast("Couldn't copy the log");
    }
  }
</script>

<Panel title="Game log" wide>
  {#if engine.crash}
    <div class="crash" role="alert">
      <strong>{engine.crash.title}</strong>
      <span>{engine.crash.hint}</span>
    </div>
  {/if}
  {#if engine.error}
    <div class="crash" role="alert">
      <strong>Couldn't start the game</strong>
      <span>{engine.error}</span>
    </div>
  {/if}

  <div class="bar">
    <span class="muted state">
      {engine.phase === 'running' ? 'Running' : engine.phase === 'installing' ? engine.label : 'Not running'}
    </span>
    <span class="grow"></span>
    {#if engine.phase === 'running'}
      <m3e-button variant="tonal" size="small" onclick={stop}>Stop game</m3e-button>
    {/if}
    <m3e-button variant="text" size="small" onclick={copy}>
      <span slot="icon"><Icon name="logs" size={18} /></span>Copy
    </m3e-button>
  </div>

  <div class="log" bind:this={box} onscroll={onScroll} role="log" aria-live="off">
    {#if lines.length === 0}
      <p class="empty muted">The game's output appears here while it runs.</p>
    {/if}
    {#each lines as l, i (i)}
      <div class="line" class:err={l.stream === 'err'}>{l.line}</div>
    {/each}
  </div>
</Panel>

<style>
  .crash {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 16px;
    margin-bottom: 10px;
    border-radius: 16px;
    background: var(--md-sys-color-error-container, #93000a);
    color: var(--md-sys-color-on-error-container, #ffdad6);
  }
  .crash span {
    font-size: 13.5px;
    user-select: text;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }
  .grow {
    flex: 1;
  }
  .state {
    font-size: 13px;
    padding-left: 4px;
  }
  .log {
    height: min(380px, calc(100vh - 300px));
    overflow: auto;
    padding: 10px 12px;
    border-radius: 16px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-lowest, #0f0d0d) 80%, transparent);
    font-size: 12.5px;
    line-height: 1.5;
    user-select: text;
    font-variant-numeric: tabular-nums;
  }
  .line {
    white-space: pre-wrap;
    word-break: break-all;
  }
  .line.err {
    color: var(--md-sys-color-error, #ffb4ab);
  }
  .empty {
    margin: 4px;
  }
</style>
