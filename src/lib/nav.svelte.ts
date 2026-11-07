export type View = 'home' | 'settings' | 'accounts' | 'instances' | 'logs' | 'profile';
export type Tab = 'minecraft' | 'launcher' | 'storage' | 'about';
export type ProfileTab = 'overview' | 'mods' | 'screenshots';

export const nav: {
  view: View;
  tab: Tab;
  createProfile: boolean;
  /** Open the profile editor for this profile id when the Profiles page mounts */
  editProfile: string;
  profileTab: ProfileTab;
} = $state({
  view: 'home',
  tab: 'minecraft',
  createProfile: false,
  editProfile: '',
  profileTab: 'overview',
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

/** The profile hub (overview, mods, screenshots) for the current profile. */
export function goProfile(tab: ProfileTab = 'overview') {
  nav.profileTab = tab;
  nav.view = 'profile';
}

/** Open the editor for one profile on the Profiles page. */
export function goEditProfile(id: string) {
  nav.editProfile = id;
  nav.view = 'instances';
}
