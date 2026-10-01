import { useCallback, useEffect, useRef, useState } from 'react';
import { Search, ArrowUpRight, Clipboard, X, AlertCircle } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api, errorText } from './api';
import type { Entry, Page, Settings } from './types';
import { TemplateComposer } from './components/TemplateComposer';
import { useDebounce, useEvent, useTheme } from './hooks';
import { EntryList } from './components/EntryList';
export function Quick() {
  const [query, setQuery] = useState('');
  const q = useDebounce(query, 80);
  const [page, setPage] = useState<Page>({ items: [], total: 0 });
  const [index, setIndex] = useState(0);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [template, setTemplate] = useState<Entry | null>(null);
  const [loading, setLoading] = useState(false);
  const [settings, setSettings] = useState<Settings>();
  const [revision, refresh] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const queryEpoch = useRef(0);
  const [tab, setTab] = useState<'quick' | 'snippets'>('quick');
  useTheme(settings?.theme);
  const updateSettings = useCallback(() => {
    api
      .settings()
      .then(setSettings)
      .catch((e) => setError(errorText(e)));
  }, []);
  useEffect(updateSettings, [updateSettings]);
  useEvent('settings-changed', updateSettings);
  useEvent(
    'changed',
    useCallback(() => refresh((n) => n + 1), []),
  );
  useEvent(
    'quick-open',
    useCallback(() => {
      setQuery('');
      setTemplate(null);
      setError('');
      setIndex(0);
      refresh((n) => n + 1);
      setTimeout(() => input.current?.focus(), 30);
    }, []),
  );
  useEffect(() => {
    let active = true;
    queryEpoch.current += 1;
    setLoading(true);
    api
      .query(q, tab, '', 'recent')
      .then((p) => {
        if (active) {
          setPage(p);
          setIndex(0);
        }
      })
      .catch((e) => {
        if (active) setError(errorText(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [q, tab, revision]);
  const selected = page.items[index];
  useEvent<string>(
    'template-open',
    useCallback((id) => {
      api
        .entry(id)
        .then(setTemplate)
        .catch((e) => setError(errorText(e)));
    }, []),
  );
  useEffect(() => {
    if (selected)
      document.querySelector(`[data-id="${selected.id}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selected]);
  async function paste(id: string) {
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      const entry = await api.entry(id);
      if (entry.library && entry.content.isTemplate) {
        setTemplate(entry);
        return;
      }
      await api.copy(id, null, true);
    } catch (e) {
      setError(errorText(e));
      await getCurrentWindow().show();
      await getCurrentWindow().setFocus();
    } finally {
      setBusy(false);
    }
  }
  return (
    <div
      className="quick"
      onKeyDown={(e) => {
        if (document.querySelector('dialog[open]')) return;
        if (e.key === 'Escape') {
          e.preventDefault();
          void getCurrentWindow().hide();
        }
        if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
          e.preventDefault();
          setIndex((i) =>
            Math.max(0, Math.min(page.items.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1))),
          );
        }
        if (e.key === 'Enter' && selected) {
          e.preventDefault();
          void paste(selected.id);
        }
      }}
    >
      <header className="quick-header">
        <span className="brand-mark">
          <Clipboard size={17} />
        </span>
        <strong>Super Clipboard</strong>
        <span className="muted">Szybkie wklejanie</span>
        <button
          className="icon-button"
          aria-label="Zamknij"
          onClick={() => void getCurrentWindow().hide()}
        >
          <X size={17} />
        </button>
      </header>
      <div className="quick-search">
        <Search size={20} />
        <input
          ref={input}
          autoFocus
          role="combobox"
          aria-autocomplete="list"
          aria-controls="entries"
          aria-expanded={true}
          aria-activedescendant={selected ? `entry-${selected.id}` : undefined}
          aria-label="Szukaj w historii i snippetach"
          placeholder="Czego potrzebujesz?"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <kbd>Esc</kbd>
      </div>
      <div className="quick-tabs">
        <button aria-pressed={tab === 'quick'} onClick={() => setTab('quick')}>
          Wszystko
        </button>
        <button aria-pressed={tab === 'snippets'} onClick={() => setTab('snippets')}>
          Snippety
        </button>
        <span>{page.total} wyników</span>
      </div>
      {error ? (
        <div className="quick-error" role="alert">
          <AlertCircle size={17} />
          {error}
        </div>
      ) : null}
      <EntryList
        items={page.items}
        selected={selected?.id || null}
        onSelect={(id) => setIndex(page.items.findIndex((e) => e.id === id))}
        onActivate={(id) => void paste(id)}
        query={query}
        total={page.total}
        onMore={async () => {
          const epoch = queryEpoch.current;
          setLoading(true);
          try {
            const more = await api.query(q, tab, '', 'recent', page.items.length);
            if (epoch === queryEpoch.current)
              setPage((old) => ({
                items: [
                  ...old.items,
                  ...more.items.filter((e) => !old.items.some((o) => o.id === e.id)),
                ],
                total: more.total,
              }));
          } catch (e) {
            setError(errorText(e));
          } finally {
            if (epoch === queryEpoch.current) setLoading(false);
          }
        }}
        loading={loading}
        quick
      />
      <footer className="quick-footer">
        <span>
          <kbd>↑</kbd>
          <kbd>↓</kbd> wybierz <kbd>↵</kbd> {busy ? 'Wklejanie…' : 'wklej'}
        </span>
        <button className="text-button" onClick={() => void api.main()}>
          Otwórz bibliotekę
          <ArrowUpRight size={15} />
        </button>
      </footer>
      {template ? (
        <TemplateComposer
          entry={template}
          paste
          onClose={() => {
            setTemplate(null);
            input.current?.focus();
          }}
          onDone={() => setTemplate(null)}
        />
      ) : null}
    </div>
  );
}
