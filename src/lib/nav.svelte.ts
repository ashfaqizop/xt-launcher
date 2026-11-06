export type View = 'home' | 'settings' | 'accounts' | 'instances' | 'logs';

export const nav: { view: View; tab: 'minecraft' | 'launcher' | 'about' } = $state({
  view: 'home',
  tab: 'minecraft',
});

export function go(view: View, tab?: 'minecraft' | 'launcher' | 'about') {
  nav.view = view;
  if (tab) nav.tab = tab;
}
