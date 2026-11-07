<script lang="ts">
  import Icon from '../lib/Icon.svelte';
  import ProfileIcon from '../lib/ProfileIcon.svelte';
  import { ACCENTS, BLOCKS, blockUri } from '../lib/blocks';
  import type { Profile } from '../lib/data.svelte';
  import { showToast } from '../lib/toast.svelte';

  let {
    profile,
    onchange,
  }: { profile: Pick<Profile, 'kind' | 'loader' | 'icon' | 'iconImage' | 'accent'>; onchange: (patch: Partial<Profile>) => void } = $props();

  let fileInput: HTMLInputElement;

  /** Crop to a square and shrink to 128px so the stored picture stays tiny. */
  async function usePicture(file: File) {
    const bitmap = await createImageBitmap(file);
    const side = Math.min(bitmap.width, bitmap.height);
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 128;
    canvas
      .getContext('2d')!
      .drawImage(bitmap, (bitmap.width - side) / 2, (bitmap.height - side) / 2, side, side, 0, 0, 128, 128);
    bitmap.close();
    onchange({ iconImage: canvas.toDataURL('image/webp', 0.86), icon: 'image' });
  }

  async function picked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    try {
      await usePicture(file);
    } catch {
      showToast('That picture could not be used');
    }
  }
</script>

<div class="picker">
  <div class="group" role="radiogroup" aria-label="Icon">
    <button type="button" role="radio" aria-checked={!profile.icon} class="opt" class:on={!profile.icon} title="Default" onclick={() => onchange({ icon: '' })}>
      <ProfileIcon profile={{ kind: profile.kind, loader: profile.loader, accent: profile.accent }} size={36} />
    </button>
    {#each BLOCKS as b (b.id)}
      <button type="button" role="radio" aria-checked={profile.icon === b.id} class="opt" class:on={profile.icon === b.id} title={b.name} onclick={() => onchange({ icon: b.id })}>
        <img src={blockUri(b.id)} alt={b.name} draggable="false" />
      </button>
    {/each}
    <button
      type="button"
      role="radio"
      aria-checked={profile.icon === 'image'}
      class="opt"
      class:on={profile.icon === 'image'}
      title="Use your own picture"
      onclick={() => (profile.iconImage && profile.icon !== 'image' ? onchange({ icon: 'image' }) : fileInput.click())}
    >
      {#if profile.icon === 'image' && profile.iconImage}
        <img class="photo" src={profile.iconImage} alt="" draggable="false" />
      {:else}
        <Icon name="addPhoto" size={22} />
      {/if}
    </button>
    <input bind:this={fileInput} type="file" accept="image/*" hidden onchange={picked} />
  </div>

  <div class="group colours" role="radiogroup" aria-label="Accent colour">
    <button
      type="button"
      role="radio"
      aria-checked={!profile.accent}
      class="swatch none"
      class:on={!profile.accent}
      title="Theme colour"
      onclick={() => onchange({ accent: '' })}
    >
      <Icon name="palette" size={16} />
    </button>
    {#each ACCENTS as a (a.id)}
      <button
        type="button"
        role="radio"
        aria-checked={profile.accent === a.hex}
        class="swatch"
        class:on={profile.accent === a.hex}
        style:--c={a.hex}
        title={a.name}
        aria-label={a.name}
        onclick={() => onchange({ accent: a.hex })}
      ></button>
    {/each}
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 45%, transparent);
  }
  .group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .opt {
    all: unset;
    box-sizing: border-box;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    cursor: pointer;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 70%, transparent);
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition:
      outline-color 120ms,
      background 120ms,
      transform 120ms;
  }
  .opt:hover {
    background: var(--md-sys-color-surface-container-highest, #383434);
    transform: translateY(-1px);
  }
  .opt.on {
    outline-color: var(--md-sys-color-primary, #c23a24);
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 55%, transparent);
  }
  .opt:focus-visible {
    outline-color: var(--md-sys-color-primary, #c23a24);
  }
  .opt img {
    width: 28px;
    height: 28px;
    image-rendering: pixelated;
    border-radius: 2px;
  }
  .opt img.photo {
    width: 36px;
    height: 36px;
    border-radius: 11px;
    image-rendering: auto;
    object-fit: cover;
  }

  .colours {
    gap: 10px;
  }
  .swatch {
    all: unset;
    box-sizing: border-box;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--c);
    cursor: pointer;
    display: grid;
    place-items: center;
    box-shadow: inset 0 0 0 2px rgb(255 255 255 / 0.12);
    transition: transform 120ms;
  }
  .swatch:hover {
    transform: scale(1.1);
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--md-sys-color-surface, #1e1b1b),
      0 0 0 4px var(--c, var(--md-sys-color-primary, #c23a24));
  }
  .swatch.none {
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
    --c: var(--md-sys-color-primary, #c23a24);
  }
  .swatch:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    outline-offset: 3px;
  }
</style>
