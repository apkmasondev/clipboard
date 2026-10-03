// Development-only UI test fixture. This file is not an entry in the production build.
import { mockIPC } from '@tauri-apps/api/mocks';
import { createRoot } from 'react-dom/client';
import { App } from '../src/App';
import { Quick } from '../src/Quick';
import { emptyContent, type Entry } from '../src/types';
import '../src/styles.css';
let entries: Entry[] = [
  {
    id: 'f30d29b8-7059-4579-85b4-39f30470bf14',
    kind: 'text',
    created: Date.now(),
    updated: Date.now(),
    bytes: 380,
    pinned: true,
    library: true,
    content: {
      ...emptyContent(),
      title: 'Przegląd kodu przed publikacją',
      category: 'Praca z AI',
      tags: ['review', 'bezpieczeństwo'],
      description: 'Ostatnie spojrzenie przed oddaniem projektu.',
      text: 'Przeanalizuj kod pod kątem błędów, bezpieczeństwa, przypadków brzegowych i czytelności.\n\nDla każdej uwagi podaj lokalizację, konkretny scenariusz błędu, jego wpływ i propozycję poprawki.\n\nOdróżnij fakty od przypuszczeń.',
    },
  },
  {
    id: 'f30d29b8-7059-4579-85b4-39f30470bf15',
    kind: 'json',
    created: Date.now() - 60000,
    updated: Date.now() - 60000,
    bytes: 96,
    pinned: false,
    library: false,
    content: {
      ...emptyContent(),
      title: '{"name": "Super Clipboard", "version": "1.0.0"}',
      text: '{"name": "Super Clipboard", "version": "1.0.0", "private": true}',
      source: 'Code.exe',
    },
  },
  {
    id: 'f30d29b8-7059-4579-85b4-39f30470bf16',
    kind: 'link',
    created: Date.now() - 120000,
    updated: Date.now() - 120000,
    bytes: 56,
    pinned: false,
    library: false,
    content: {
      ...emptyContent(),
      title: 'https://v2.tauri.app/start/',
      text: 'https://v2.tauri.app/start/',
      source: 'msedge.exe',
    },
  },
  {
    id: 'f30d29b8-7059-4579-85b4-39f30470bf17',
    kind: 'text',
    created: Date.now() - 180000,
    updated: Date.now() - 180000,
    bytes: 286,
    pinned: false,
    library: false,
    content: {
      ...emptyContent(),
      title: 'Pomysły na następne wydanie',
      text: 'Sprawdzić obsługę klawiatury.\nUprościć panel podglądu.\nDopracować jasny motyw.',
      source: 'notepad.exe',
    },
  },
];
let settings = {
  theme: new URLSearchParams(location.search).get('theme') || 'dark',
  maxItems: 2000,
  maxDays: 30,
  maxMb: 100,
  blockedApps: ['1password.exe', 'keepassxc.exe', 'bitwarden.exe'],
  disabledTypes: [],
  skipSecrets: true,
  blockUnknown: false,
  paused: false,
  autostart: false,
  quickShortcut: 'Control+Shift+KeyV',
};
// Synthetic, public images for visual regression checks; never read the user's clipboard.
const imageSamples = new Map<string, string>();
if (new URLSearchParams(location.search).has('images')) {
  entries = Array.from({ length: 16 }, (_, index): Entry => {
    const id = `f30d29b8-7059-4579-85b4-${String(index).padStart(12, '0')}`;
    const canvas = document.createElement('canvas');
    canvas.width = index % 3 === 1 ? 180 : 512;
    canvas.height = index % 3 === 2 ? 160 : 320;
    const ctx = canvas.getContext('2d')!;
    if (index % 3 !== 2) {
      ctx.fillStyle = ['#173b42', '#decaa8', '#e5ded0', '#485464'][index % 4];
      ctx.fillRect(0, 0, canvas.width, canvas.height);
    }
    ctx.fillStyle = ['#84d4bb', '#1d3341', '#ad7045', '#e4ceb0'][index % 4];
    ctx.fillRect(20, 24, canvas.width - 40, 18);
    ctx.beginPath();
    ctx.arc(
      canvas.width / 2,
      canvas.height / 2 + 18,
      Math.min(canvas.width, canvas.height) / 4,
      0,
      Math.PI * 2,
    );
    ctx.fill();
    ctx.fillStyle = '#fff';
    ctx.font = 'bold 38px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(String(index + 1).padStart(2, '0'), canvas.width / 2, canvas.height / 2 + 30);
    imageSamples.set(id, canvas.toDataURL('image/png'));
    return {
      id,
      kind: 'image',
      library: false,
      pinned: index === 0,
      created: Date.now() - index * 60000,
      updated: Date.now() - index * 60000,
      bytes: 45000 + index * 9000,
      content: {
        ...emptyContent(),
        title: `Obraz · ${canvas.width} × ${canvas.height}`,
        width: canvas.width,
        height: canvas.height,
        source: 'mspaint.exe',
      },
    };
  });
}
mockIPC((cmd, args) => {
  const a = args as Record<string, unknown>;
  switch (cmd) {
    case 'get_settings':
      return settings;
    case 'save_settings':
      settings = a.settings as typeof settings;
      return;
    case 'get_stats':
      return {
        history: entries.filter((e) => !e.library).length,
        snippets: entries.filter((e) => e.library).length,
        pinned: 1,
        bytes: 920,
        diskBytes: 49152,
      };
    case 'get_thumbnail':
    case 'get_image':
      if (String(a.id).endsWith('000000000015'))
        return Promise.reject('Przykładowy błąd odczytu obrazu.');
      return new Promise((resolve) =>
        setTimeout(() => resolve(imageSamples.get(String(a.id))), 100),
      );
    case 'get_warnings':
      return [];
    case 'query_entries': {
      const es = entries
        .filter((e) =>
          a.view === 'snippets'
            ? e.library
            : a.view === 'favorites'
              ? e.pinned
              : a.view === 'quick'
                ? true
                : !e.library,
        )
        .filter((e) => !a.kind || a.kind === e.kind)
        .filter((e) =>
          JSON.stringify(e.content).toLowerCase().includes(String(a.query).toLowerCase()),
        );
      return {
        items: es.map((e) => ({ ...e, ...e.content, preview: e.content.text })),
        total: es.length,
      };
    }
    case 'get_entry':
      return entries.find((e) => e.id === a.id);
    case 'save_snippet': {
      const e = {
        id: String(a.id || crypto.randomUUID()),
        kind: 'text',
        library: true,
        pinned: a.pinned,
        bytes: 100,
        created: Date.now(),
        updated: Date.now(),
        content: a.content,
      } as Entry;
      entries = [e, ...entries.filter((x) => x.id !== e.id)];
      return e;
    }
    case 'pin_entry':
      entries = entries.map((e) => (e.id === a.id ? { ...e, pinned: !e.pinned } : e));
      return;
    case 'delete_entry':
      entries = entries.filter((e) => e.id !== a.id);
      return;
    case 'copy_entry':
      return;
    default:
      return null;
  }
});
createRoot(document.getElementById('root')!).render(
  new URLSearchParams(location.search).has('quick') ? <Quick /> : <App />,
);
