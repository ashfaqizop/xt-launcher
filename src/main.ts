import '@fontsource-variable/bricolage-grotesque/wght.css';

// M3E components used by the launcher (each import registers its custom elements)
import '@m3e/web/theme';
import '@m3e/web/avatar';
import '@m3e/web/button';
import '@m3e/web/dialog';
import '@m3e/web/divider';
import '@m3e/web/form-field';
import '@m3e/web/icon-button';
import '@m3e/web/menu';
import '@m3e/web/option';
import '@m3e/web/progress-indicator';
import '@m3e/web/segmented-button';
import '@m3e/web/select';
import '@m3e/web/slider';
import '@m3e/web/switch';
import '@m3e/web/tabs';
import '@m3e/web/tooltip';

import './styles/base.css';
import './styles/app.css';

import { mount } from 'svelte';
import App from './App.svelte';

// m3e-theme must be a direct child of <body>: only then does it theme pop-ups that
// render in the top layer (select panels, menus).
const app = mount(App, { target: document.body });

export default app;
