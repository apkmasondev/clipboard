import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Clipboard,
  History,
  Bookmark,
  Pin,
  Image,
  Code2,
  Link,
  Settings as SettingsIcon,
  Search,
  Plus,
  Pause,
  Play,
  ShieldCheck,
  Download,
  Upload,
  X,
  Check,
  AlertCircle,
  ChevronDown,
} from 'lucide-react';
import { api, errorText, shortcutLabel } from './api';
import type { Entry, Page, Settings, Stats, View } from './types';
import { kinds } from './types';
import { useDebounce, useEvent, useTheme } from './hooks';
import { EntryList } from './components/EntryList';
import { Detail } from './components/Detail';
import { SnippetEditor } from './components/SnippetEditor';
import { SettingsPanel } from './components/SettingsPanel';
import { Modal } from './components/Modal';
import { TemplateComposer } from './components/TemplateComposer';
import { BackupDialog } from './components/BackupDialog';
const nav = [
  { id: 'history', label: 'Historia', icon: History },
  { id: 'snippets', label: 'Snippety', icon: Bookmark },
  { id: 'favorites', label: 'Ulubione', icon: Pin },
  { id: 'images', label: 'Obrazy', icon: Image },
  { id: 'code', label: 'Kod i JSON', icon: Code2 },
  { id: 'links', label: 'Linki', icon: Link },
] as const;
const subtitles: Partial<Record<View, string>> = {
  history: 'To, co kopiujesz. Gotowe do ponownego użycia.',
  snippets: 'Twoje słowa, prompty i polecenia. Na stałe.',
  favorites: 'Rzeczy, do których warto wracać.',
  images: 'Obrazy i zrzuty ekranu w jednym miejscu.',
  code: 'Fragmenty kodu i uporządkowane dane.',
  links: 'Adresy, które chcesz mieć pod ręką.',
};
export function App() {
  const [view, setView] = useState<View>('history');
  const [query, setQuery] = useState('');
  const q = useDebounce(query);
  const [kind, setKind] = useState('');
  const [sort, setSort] = useState('recent');
  const [page, setPage] = useState<Page>({ items: [], total: 0 });
  const [selected, setSelected] = useState<string | null>(null);
  const [entry, setEntry] = useState<Entry | null>(null);
  const [template, setTemplate] = useState<Entry | null>(null);
  const [backup, setBackup] = useState(false);
  const [loading, setLoading] = useState(false);
  const [settings, setSettings] = useState<Settings>();
  const [stats, setStats] = useState<Stats | null>(null);
  const [revision, refresh] = useState(0);
  const [editor, setEditor] = useState<{ entry?: Entry; duplicate?: boolean } | null>(null);
  const [confirm, setConfirm] = useState<{
    title: string;
    text: string;
    action: () => Promise<unknown>;
    label: string;
  } | null>(null);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const [confirmError, setConfirmError] = useState('');
  const settingsDirty = useRef(false);
  const onSettingsDirty = useCallback((dirty: boolean) => {
    settingsDirty.current = dirty;
  }, []);
  useEffect(() => setConfirmError(''), [confirm]);
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [loadError, setLoadError] = useState('');
  const search = useRef<HTMLInputElement>(null);
  const queryEpoch = useRef(0);
  useTheme(settings?.theme);
  const notify = useCallback((text: string, error = false) => setToast({ text, error }), []);
  useEffect(() => {
    if (toast && !toast.error) {
      const t = setTimeout(() => setToast(null), 3000);
      return () => clearTimeout(t);
    }
  }, [toast]);
  const updateSettings = useCallback(() => {
    api
      .settings()
      .then(setSettings)
      .catch((e) => notify(errorText(e), true));
  }, [notify]);
  useEffect(() => {
    updateSettings();
    api
      .warnings()
      .then(setWarnings)
      .catch((e) => notify(errorText(e), true));
  }, [updateSettings, notify]);
  useEvent('settings-changed', updateSettings);
  useEvent(
    'changed',
    useCallback(() => refresh((n) => n + 1), []),
  );
  useEvent<string>(
    'notice',
    useCallback((s) => notify(s, true), [notify]),
  );
  useEffect(() => {
    api
      .stats()
      .then(setStats)
      .catch((e) => notify(errorText(e), true));
  }, [revision, notify]);
  useEffect(() => {
    queryEpoch.current += 1;
    if (view === 'settings') return;
    let active = true;
    setLoading(true);
    setLoadError('');
    api
      .query(q, view, kind, sort)
      .then((p) => {
        if (active) {
          setPage(p);
          setSelected((id) => (p.items.some((e) => e.id === id) ? id : p.items[0]?.id || null));
        }
      })
      .catch((e) => {
        if (active) setLoadError(errorText(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [q, view, kind, sort, revision]);
  useEffect(() => {
    let active = true;
    setEntry((previous) => (previous?.id === selected ? previous : null));
    if (selected)
      api
        .entry(selected)
        .then((e) => {
          if (active) setEntry(e);
        })
        .catch((e) => {
          if (active) notify(errorText(e), true);
        });
    return () => {
      active = false;
    };
  }, [selected, revision, notify]);
  useEffect(() => {
    if (selected)
      document.querySelector(`[data-id="${selected}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selected]);
  const copy = useCallback(
    async (id: string, text?: string) => {
      try {
        if (text === undefined) {
          const e = await api.entry(id);
          if (e.library && e.content.isTemplate) {
            setTemplate(e);
            return;
          }
        }
        await api.copy(id, text ?? null);
        notify('Skopiowano. Gotowe do wklejenia.');
      } catch (e) {
        notify(errorText(e), true);
      }
    },
    [notify],
  );
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (document.querySelector('dialog[open]')) return;
      if (e.ctrlKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        search.current?.focus();
      }
      if (e.ctrlKey && e.key.toLowerCase() === 'n') {
        e.preventDefault();
        setEditor({});
      }
      const target = e.target as HTMLElement;
      if (target.matches('textarea,input,select') || window.getSelection()?.toString()) return;
      if (e.ctrlKey && e.key.toLowerCase() === 'c' && selected) {
        e.preventDefault();
        void copy(selected);
      }
      if (e.key === 'Enter' && selected && target.getAttribute('role') === 'option') {
        e.preventDefault();
        void copy(selected);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [selected, copy]);
  async function pin(id: string) {
    try {
      await api.pin(id);
    } catch (e) {
      notify(errorText(e), true);
    }
  }
  function remove(e: Entry) {
    setConfirm({
      title: e.library ? 'Usunąć snippet?' : 'Usunąć element?',
      text: `„${e.content.title.slice(0, 100)}” zostanie trwale usunięty. Tej operacji nie można cofnąć.`,
      label: 'Usuń',
      action: () => api.delete(e.id),
    });
  }
  function clear() {
    setConfirm({
      title: 'Wyczyścić historię?',
      text: 'Wszystkie nieprzypięte wpisy historii zostaną trwale usunięte. Ulubione i biblioteka snippetów pozostaną.',
      label: 'Wyczyść historię',
      action: () => api.clear(),
    });
  }
  function navigate(next: View) {
    if (view === 'settings' && next !== view && settingsDirty.current) {
      setConfirm({
        title: 'Porzucić zmiany ustawień?',
        text: 'Masz niezapisane zmiany. Wróć i zapisz je lub opuść ustawienia bez zapisu.',
        label: 'Porzuć zmiany',
        action: async () => {
          settingsDirty.current = false;
          setView(next);
          setQuery('');
          setKind('');
          setSelected(null);
        },
      });
      return;
    }
    setView(next);
    setQuery('');
    setKind('');
    setSelected(null);
  }
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">
            <Clipboard size={21} />
          </span>
          <div>
            Super<span>Clipboard</span>
          </div>
        </div>
        <span className="nav-caption">TWOJA PRZESTRZEŃ</span>
        <nav aria-label="Główna nawigacja">
          {nav.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={view === id ? 'current' : ''}
              aria-current={view === id ? 'page' : undefined}
              onClick={() => navigate(id)}
            >
              <Icon size={18} />
              <span>{label}</span>
              {id === 'history' && stats ? (
                <span className="nav-count">{stats.history}</span>
              ) : id === 'snippets' && stats ? (
                <span className="nav-count">{stats.snippets}</span>
              ) : null}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="quick-tip">
            <span>SZYBKIE WKLEJANIE</span>
            <kbd>{shortcutLabel(settings?.quickShortcut || 'Ctrl+Shift+V')}</kbd>
            <p>Twój schowek w każdej aplikacji.</p>
          </div>
          <button
            className={`settings-nav ${view === 'settings' ? 'current' : ''}`}
            onClick={() => navigate('settings')}
          >
            <SettingsIcon size={18} />
            Ustawienia
          </button>
          <div className="local-status">
            <ShieldCheck size={14} />
            <span>Tylko na tym komputerze</span>
          </div>
        </div>
      </aside>
      <main className="workspace">
        {warnings.map((w, i) => (
          <div className="warning-banner" key={i}>
            <AlertCircle size={16} />
            {w}
            <button
              className="text-button"
              onClick={() => {
                navigate('settings');
                setWarnings([]);
              }}
            >
              Zmień skrót
            </button>
          </div>
        ))}
        {view === 'settings' ? (
          settings ? (
            <SettingsPanel
              onDirtyChange={onSettingsDirty}
              key={JSON.stringify(settings)}
              settings={settings}
              stats={stats}
              onSaved={updateSettings}
              onClear={clear}
              onBackup={() => setBackup(true)}
              notify={notify}
            />
          ) : (
            <div className="empty">Wczytywanie ustawień…</div>
          )
        ) : (
          <>
            <header className="workspace-header">
              <div>
                <span className="eyebrow">SUPER CLIPBOARD</span>
                <h1>{nav.find((n) => n.id === view)?.label}</h1>
                <p>{subtitles[view]}</p>
              </div>
              <div className="header-actions">
                {view === 'snippets' ? (
                  <>
                    <button title="Szyfrowana kopia biblioteki" onClick={() => setBackup(true)}>
                      Kopia zapasowa
                    </button>
                    <button
                      className="icon-button"
                      title="Importuj snippety"
                      aria-label="Importuj snippety"
                      onClick={async () => {
                        try {
                          const n = await api.import();
                          if (n) notify(`Zaimportowano snippetów: ${n}.`);
                        } catch (e) {
                          notify(errorText(e), true);
                        }
                      }}
                    >
                      <Download size={18} />
                    </button>
                    <button
                      className="icon-button"
                      title="Eksportuj snippety"
                      aria-label="Eksportuj snippety"
                      onClick={() =>
                        setConfirm({
                          title: 'Eksportować bibliotekę?',
                          text: 'Plik JSON będzie zawierać jawną treść Twoich snippetów. Przechowuj go w bezpiecznym miejscu.',
                          label: 'Wybierz plik',
                          action: async () => {
                            if (await api.export()) notify('Wyeksportowano bibliotekę.');
                          },
                        })
                      }
                    >
                      <Upload size={18} />
                    </button>
                  </>
                ) : (
                  <button
                    className={`capture-status ${settings?.paused ? 'paused' : ''}`}
                    title="Wstrzymaj lub wznów zapisywanie"
                    onClick={async () => {
                      if (settings)
                        try {
                          await api.saveSettings({ ...settings, paused: !settings.paused });
                        } catch (e) {
                          notify(errorText(e), true);
                        }
                    }}
                  >
                    {settings?.paused ? <Pause size={14} /> : <span className="status-dot" />}
                    {settings?.paused ? 'Wstrzymano' : 'Historia aktywna'}
                    {!settings?.paused ? <Pause size={13} /> : <Play size={13} />}
                  </button>
                )}
                <button className="primary" onClick={() => setEditor({})}>
                  <Plus size={17} />
                  Nowy snippet
                </button>
              </div>
            </header>
            <div className="content-grid">
              <section
                className="list-panel"
                onKeyDown={(e) => {
                  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                    e.preventDefault();
                    const idx = page.items.findIndex((i) => i.id === selected);
                    const next =
                      page.items[
                        Math.max(
                          0,
                          Math.min(page.items.length - 1, idx + (e.key === 'ArrowDown' ? 1 : -1)),
                        )
                      ];
                    if (next) {
                      setSelected(next.id);
                      if ((e.target as HTMLElement).getAttribute('role') === 'option')
                        document
                          .querySelector<HTMLButtonElement>(`[data-id="${next.id}"]`)
                          ?.focus();
                    }
                  }
                }}
              >
                <div className="list-controls">
                  <div className="search-field">
                    <Search size={18} />
                    <input
                      ref={search}
                      aria-label="Szukaj treści, tagów lub aplikacji"
                      placeholder="Szukaj treści, tagów…"
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                    />
                    {query ? (
                      <button
                        className="icon-button"
                        aria-label="Wyczyść wyszukiwanie"
                        onClick={() => setQuery('')}
                      >
                        <X size={14} />
                      </button>
                    ) : (
                      <kbd>Ctrl F</kbd>
                    )}
                  </div>
                  <div className="list-filters">
                    <select
                      aria-label="Filtr typu"
                      value={kind}
                      onChange={(e) => setKind(e.target.value)}
                    >
                      <option value="">Wszystkie typy</option>
                      {Object.entries(kinds).map(([k, label]) => (
                        <option key={k} value={k}>
                          {label}
                        </option>
                      ))}
                    </select>
                    <select
                      aria-label="Sortowanie"
                      value={sort}
                      onChange={(e) => setSort(e.target.value)}
                    >
                      <option value="recent">Najnowsze</option>
                      <option value="oldest">Najstarsze</option>
                      <option value="title">Nazwa A–Z</option>
                      <option value="size">Największe</option>
                    </select>
                  </div>
                </div>
                <div className="list-caption">
                  <span>
                    {q
                      ? 'WYNIKI WYSZUKIWANIA'
                      : view === 'snippets'
                        ? 'BIBLIOTEKA'
                        : 'ZAPISANE ELEMENTY'}
                  </span>
                  <span>{loading ? '…' : page.total}</span>
                </div>
                {loadError ? (
                  <div className="error-box" role="alert">
                    {loadError}
                    <button onClick={() => refresh((n) => n + 1)}>Spróbuj ponownie</button>
                  </div>
                ) : null}
                <EntryList
                  items={page.items}
                  selected={selected}
                  onSelect={setSelected}
                  onActivate={(id) => void copy(id)}
                  loading={loading}
                  query={query}
                  total={page.total}
                  onMore={async () => {
                    const epoch = queryEpoch.current;
                    setLoading(true);
                    try {
                      const p = await api.query(q, view, kind, sort, page.items.length);
                      if (epoch === queryEpoch.current)
                        setPage((old) => ({
                          items: [
                            ...old.items,
                            ...p.items.filter((e) => !old.items.some((o) => o.id === e.id)),
                          ],
                          total: p.total,
                        }));
                    } catch (e) {
                      notify(errorText(e), true);
                    } finally {
                      if (epoch === queryEpoch.current) setLoading(false);
                    }
                  }}
                />
                <footer className="list-footer">
                  <span>↑ ↓ wybierz</span>
                  <span>Enter kopiuj</span>
                  <ChevronDown size={12} />
                </footer>
              </section>
              <Detail
                entry={entry}
                onCopy={(id, t) => void copy(id, t)}
                onPin={(id) => void pin(id)}
                onDelete={remove}
                onEdit={(e, duplicate) => setEditor({ entry: e, duplicate })}
                onTags={() => refresh((n) => n + 1)}
                notify={notify}
              />
            </div>
          </>
        )}
      </main>
      {toast ? (
        <div
          className={`toast ${toast.error ? 'error' : ''}`}
          role={toast.error ? 'alert' : 'status'}
        >
          {toast.error ? <AlertCircle size={18} /> : <Check size={18} />}
          <span>{toast.text}</span>
          <button
            className="icon-button"
            aria-label="Zamknij komunikat"
            onClick={() => setToast(null)}
          >
            <X size={16} />
          </button>
        </div>
      ) : null}
      {editor ? (
        <SnippetEditor
          entry={editor.entry}
          duplicate={editor.duplicate}
          onClose={() => setEditor(null)}
          onSaved={(e) => {
            setEditor(null);
            navigate('snippets');
            setSelected(e.id);
            refresh((n) => n + 1);
            notify('Snippet zapisany.');
          }}
        />
      ) : null}
      {template ? (
        <TemplateComposer
          entry={template}
          paste={false}
          onClose={() => setTemplate(null)}
          onDone={() => {
            setTemplate(null);
            notify('Gotowy tekst skopiowany.');
          }}
        />
      ) : null}
      {backup ? (
        <BackupDialog
          onClose={() => setBackup(false)}
          onDone={(message) => {
            setBackup(false);
            refresh((n) => n + 1);
            notify(message);
          }}
        />
      ) : null}
      {confirm ? (
        <Modal
          title={confirm.title}
          onClose={() => {
            if (!confirmBusy) setConfirm(null);
          }}
        >
          <div className="modal-body">
            <p>{confirm.text}</p>
            {confirmError ? (
              <p className="error-box" role="alert">
                {confirmError}
              </p>
            ) : null}
          </div>
          <footer>
            <button disabled={confirmBusy} onClick={() => setConfirm(null)}>
              Anuluj
            </button>
            <button
              className={confirm.label === 'Wybierz plik' ? 'primary' : 'danger'}
              disabled={confirmBusy}
              onClick={async () => {
                setConfirmBusy(true);
                setConfirmError('');
                try {
                  await confirm.action();
                  setConfirm(null);
                  refresh((n) => n + 1);
                } catch (e) {
                  setConfirmError(errorText(e));
                } finally {
                  setConfirmBusy(false);
                }
              }}
            >
              {confirmBusy ? 'Proszę czekać…' : confirm.label}
            </button>
          </footer>
        </Modal>
      ) : null}
    </div>
  );
}
