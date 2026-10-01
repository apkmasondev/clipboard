import { invoke } from '@tauri-apps/api/core';
import type { Content, Entry, Page, Settings, Stats, View } from './types';
export const api = {
  query: (query: string, view: View, kind: string, sort: string, offset = 0) =>
    invoke<Page>('query_entries', { query, view, kind, sort, offset }),
  entry: (id: string) => invoke<Entry>('get_entry', { id }),
  templateFields: (id: string) => invoke<string[]>('template_fields', { id }),
  renderTemplate: (id: string, values: Record<string, string>) =>
    invoke<string>('render_template', { id, values }),
  revisions: (id: string) => invoke<{ id: string; saved: number }[]>('get_revisions', { id }),
  revision: (id: string, revision: string) => invoke<Content>('get_revision', { id, revision }),
  backup: (password: string, restore: boolean) =>
    invoke<number | null>('backup_library', { password, restore }),
  image: (id: string) => invoke<string>('get_image', { id }),
  copy: (id: string, plain: string | null = null, paste = false) =>
    invoke<void>('copy_entry', { id, plain, paste }),
  pin: (id: string) => invoke<void>('pin_entry', { id }),
  tags: (id: string, tags: string[]) => invoke<void>('tag_entry', { id, tags }),
  delete: (id: string) => invoke<void>('delete_entry', { id }),
  save: (id: string | null, content: Content, pinned: boolean) =>
    invoke<Entry>('save_snippet', { id, content, pinned }),
  settings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  stats: () => invoke<Stats>('get_stats'),
  warnings: () => invoke<string[]>('get_warnings'),
  clear: () => invoke<number>('clear_history'),
  export: () => invoke<boolean>('export_snippets'),
  import: () => invoke<number>('import_snippets'),
  main: () => invoke<void>('open_main'),
};
export const errorText = (e: unknown) =>
  typeof e === 'string'
    ? e
    : e instanceof Error
      ? e.message
      : 'Nie udało się wykonać operacji. Spróbuj ponownie.';
export function size(bytes: number) {
  return bytes < 1024
    ? `${bytes} B`
    : bytes < 1048576
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / 1048576).toFixed(1)} MB`;
}
export function time(timestamp: number) {
  return new Intl.DateTimeFormat('pl-PL', {
    hour: '2-digit',
    minute: '2-digit',
    day: 'numeric',
    month: 'short',
  }).format(timestamp);
}
export function shortcutLabel(s: string) {
  return s
    .replaceAll('Control', 'Ctrl')
    .replaceAll('Key', '')
    .replaceAll('Digit', '')
    .replaceAll('+', ' + ');
}
