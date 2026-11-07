<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { go } from '../lib/nav.svelte';
  import { formatPlaytime } from '../lib/data.svelte';
  import { STEP_LABEL, cancelLaunch, engine, stop } from '../lib/engine.svelte';

  // A ticking clock for elapsed / played time
  let now = $state(Date.now());
  const tick = setInterval(() => {
    if (document.visibilityState !== 'hidden') now = Date.now();
  }, 1000);
  onDestroy(() => clearInterval(tick));

  // Keep the summary of a finished session visible for a while, errors until dismissed
  let dismissedAt = $state(0);
  const last = $derived(engine.last);
  const lastIsProblem = $derived(!!last && (!!last.crash || !!last.error));
  const showLast = $derived(
    engine.phase === 'idle' &&
      !!last &&
      last.at > dismissedAt &&
      (lastIsProblem || now - last.at < 6000),
  );
  const open = $derived(engine.phase !== 'idle' || showLast);
  $effect(() => {
    engine.ribbonOpen = open;
  });

  const clock = (ms: number) => {
    const t = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(t / 3600);
    const m = Math.floor((t % 3600) / 60);
    const s = String(t % 60).padStart(2, '0');
    return h ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
  };
  const size = (bytes: number) =>
    bytes >= 1024 ** 3
      ? `${(bytes / 1024 ** 3).toFixed(2)} GB`
      : bytes >= 1024 ** 2
        ? `${(bytes / 1024 ** 2).toFixed(1)} MB`
        : `${Math.max(1, Math.round(bytes / 1024))} KB`;
  const fmt = (n: number) => n.toLocaleString('en-US');

  const detail = $derived.by(() => {
    const parts: string[] = [];
    if (engine.total > 1) parts.push(`${fmt(engine.done)} / ${fmt(engine.total)} files`);
    if (engine.bytes > 0) parts.push(size(engine.bytes));
    if (engine.speed > 50 * 1024) parts.push(`${size(engine.speed)}/s`);
    if (engine.startedAt) parts.push(clock(now - engine.startedAt));
    return parts.join('  ·  ');
  });
  const determinate = $derived(engine.total > 1 && engine.percent > 0);

  function dismiss() {
    dismissedAt = Date.now();
  }
</script>

<div class="ribbon glass" class:open class:problem={showLast && lastIsProblem} role="status" aria-live="polite" aria-hidden={!open}>
  {#if engine.phase === 'installing'}
    <div class="top">
      <div class="lead">
        <span class="badge"><Icon name="rocket" size={20} /></span>
        <div class="words">
          <div class="title">
            {engine.label}{determinate ? ` · ${Math.floor(engine.percent)}%` : ''}
          </div>
          <div class="sub muted">{engine.repairing ? "Repairing " : ""}{engine.profileName}{detail ? `  ·  ${detail}` : ''}</div>
        </div>
      </div>
      <div class="actions">
        <m3e-button variant="text" size="small" onclick={() => go('logs')}>Log</m3e-button>
        <m3e-button variant="tonal" size="small" disabled={engine.cancelling} onclick={cancelLaunch}>
          {engine.cancelling ? 'Cancelling…' : 'Cancel'}
        </m3e-button>
      </div>
    </div>

    <m3e-linear-progress-indicator
      class="bar"
      variant="wavy"
      mode={determinate ? 'determinate' : 'indeterminate'}
      value={engine.percent}
      max="100"
    ></m3e-linear-progress-indicator>

    {#if engine.steps.length}
      <ol class="steps" aria-label="Steps">
        {#each engine.steps as s (s)}
          <li class:now={s === engine.stage} class:done={s !== engine.stage}>
            {#if s !== engine.stage}<Icon name="check" size={14} />{/if}
            {STEP_LABEL[s]}
          </li>
        {/each}
      </ol>
    {/if}
  {:else if engine.phase === 'running'}
    <div class="top">
      <div class="lead">
        <span class="badge live"><Icon name="play" size={20} /></span>
        <div class="words">
          <div class="title">Playing {engine.profileName}</div>
          <div class="sub muted">{clock(now - engine.runningSince)} this session</div>
        </div>
      </div>
      <div class="actions">
        <m3e-button variant="text" size="small" onclick={() => go('logs')}>Game log</m3e-button>
        <m3e-button variant="tonal" size="small" onclick={stop}>Stop</m3e-button>
      </div>
    </div>
  {:else if showLast && last}
    <div class="top">
      <div class="lead">
        <span class="badge" class:bad={lastIsProblem}>
          <Icon name={lastIsProblem ? 'info' : 'check'} size={20} />
        </span>
        <div class="words">
          {#if last.crash}
            <div class="title">{last.crash.title}</div>
            <div class="sub">{last.crash.hint}</div>
          {:else if last.error}
            <div class="title">Couldn't start {last.profileName}</div>
            <div class="sub clamp">{last.error}</div>
          {:else if last.repaired}
            <div class="title">Profile repaired</div>
            <div class="sub muted">{last.profileName}  ·  every file was checked</div>
          {:else if last.cancelled}
            <div class="title">Launch cancelled</div>
            <div class="sub muted">Downloaded files are kept for next time</div>
          {:else}
            <div class="title">Session ended</div>
            <div class="sub muted">
              {last.profileName}{last.playedMs ? `  ·  played ${formatPlaytime(last.playedMs)}` : ''}
            </div>
          {/if}
        </div>
      </div>
      <div class="actions">
        {#if lastIsProblem}
          <m3e-button variant="tonal" size="small" onclick={() => go('logs')}>View log</m3e-button>
        {/if}
        <m3e-icon-button size="small" aria-label="Dismiss" onclick={dismiss}>
          <Icon name="close" size={18} />
        </m3e-icon-button>
      </div>
    </div>
  {/if}
</div>

<style>
  .ribbon {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: 16px;
    z-index: 25;
    padding: 12px 14px 12px 12px;
    border-radius: 22px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transform: translateY(calc(100% + 24px));
    opacity: 0;
    pointer-events: none;
    transition:
      transform 420ms cubic-bezier(0.2, 0, 0, 1),
      opacity 240ms ease;
  }
  .ribbon.open {
    transform: none;
    opacity: 1;
    pointer-events: auto;
  }
  .ribbon.problem {
    background: color-mix(in srgb, var(--md-sys-color-error-container, #93000a) 85%, transparent);
    color: var(--md-sys-color-on-error-container, #ffdad6);
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .lead {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .badge {
    width: 40px;
    height: 40px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .badge.live {
    background: var(--md-sys-color-tertiary-container, #5b4a1f);
    color: var(--md-sys-color-on-tertiary-container, #ffe08c);
  }
  .badge.bad {
    background: var(--md-sys-color-error, #ffb4ab);
    color: var(--md-sys-color-on-error, #690005);
  }
  .words {
    min-width: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub.clamp {
    white-space: normal;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    user-select: text;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .bar {
    display: block;
    width: 100%;
    --m3e-linear-progress-indicator-thickness: 5px;
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .steps li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: 999px;
    font-size: 11.5px;
    font-weight: 600;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 70%, transparent);
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .steps li.now {
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }

  @media (prefers-reduced-motion: reduce) {
    .ribbon {
      transition: opacity 120ms ease;
    }
  }
</style>
