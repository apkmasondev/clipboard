export type Kind =
  'text' | 'rich' | 'link' | 'path' | 'code' | 'json' | 'color' | 'image' | 'files';
export type View =
  'history' | 'snippets' | 'favorites' | 'images' | 'code' | 'links' | 'settings' | 'quick';
export interface Content {
  title: string;
  text: string;
  description: string;
  category: string;
  tags: string[];
  shortcut: string;
  isTemplate: boolean;
  source: string;
  html: number[];
  rtf: number[];
  files: string[];
  width: number;
  height: number;
}
export interface Entry {
  id: string;
  kind: Kind;
  created: number;
  updated: number;
  bytes: number;
  pinned: boolean;
  library: boolean;
  content: Content;
}
export interface Summary {
  id: string;
  kind: Kind;
  created: number;
  updated: number;
  bytes: number;
  pinned: boolean;
  library: boolean;
  title: string;
  preview: string;
  category: string;
  tags: string[];
  source: string;
  shortcut: string;
  isTemplate: boolean;
  width: number;
  height: number;
}
export interface Settings {
  theme: 'system' | 'light' | 'dark';
  maxItems: number;
  maxDays: number;
  maxMb: number;
  blockedApps: string[];
  disabledTypes: Kind[];
  skipSecrets: boolean;
  blockUnknown: boolean;
  paused: boolean;
  autostart: boolean;
  quickShortcut: string;
}
export interface Stats {
  history: number;
  snippets: number;
  pinned: number;
  bytes: number;
  diskBytes: number;
}
export interface Page {
  items: Summary[];
  total: number;
}
export const kinds: Record<Kind, string> = {
  text: 'Tekst',
  rich: 'Tekst z formatowaniem',
  link: 'Link',
  path: 'Ścieżka',
  code: 'Kod',
  json: 'JSON',
  color: 'Kolor',
  image: 'Obraz',
  files: 'Pliki',
};
export const emptyContent = (): Content => ({
  title: '',
  text: '',
  description: '',
  category: '',
  tags: [],
  shortcut: '',
  isTemplate: false,
  source: '',
  html: [],
  rtf: [],
  files: [],
  width: 0,
  height: 0,
});
