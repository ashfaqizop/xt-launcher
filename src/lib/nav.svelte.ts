export type View = 'home' | 'settings' | 'accounts' | 'instances' | 'logs';
type Tab = 'minecraft' | 'launcher' | 'about';

export const nav: { view: View; tab: Tab; createProfile: boolean } = $state({
  view: 'home',
  tab: 'minecraft',
  createProfile: false,
});

export function go(view: View, tab?: Tab) {
  nav.view = view;
  if (tab) nav.tab = tab;
}

/** Open the Profiles page straight on the New profile chooser. */
export function goCreateProfile() {
  nav.createProfile = true;
  nav.view = 'instances';
}
