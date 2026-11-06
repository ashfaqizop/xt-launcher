<script lang="ts">
  import flame from './assets/flame.png';
</script>

<m3e-theme color="#c23a24" scheme="dark">
  <main>
    <img src={flame} alt="" draggable="false" />
    <h1>XT Launcher</h1>
    <p>Phase 1 scaffold. The launcher UI lands in Phase 3.</p>
  </main>
</m3e-theme>

<style>
  m3e-theme {
    display: block;
    height: 100%;
  }
  main {
    height: 100vh;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 8px;
    color: var(--md-sys-color-on-surface, #ece0dd);
    background: var(--md-sys-color-surface, #100f0f);
  }
  img {
    height: 72px;
    width: auto;
  }
  h1 {
    margin: 8px 0 0;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
</style>
