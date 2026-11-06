import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// M3E custom elements (buttons, tabs, ...) are keyboard accessible and fire `click`
// for Enter/Space themselves; Svelte's a11y heuristics only know native elements.
const IGNORED = new Set([
  'a11y_click_events_have_key_events',
  'a11y_no_static_element_interactions',
  'a11y_label_has_associated_control',
]);

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    warningFilter: (warning) => !IGNORED.has(warning.code),
  },
};
