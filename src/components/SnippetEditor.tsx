import { useState } from 'react';
import { Pin, Save } from 'lucide-react';
import type { Entry } from '../types';
import { emptyContent } from '../types';
import { api, errorText, time } from '../api';
import { Modal } from './Modal';
export function SnippetEditor({
  entry,
  duplicate,
  onClose,
  onSaved,
}: {
  entry?: Entry;
  duplicate?: boolean;
  onClose: () => void;
  onSaved: (e: Entry) => void;
}) {
  const [content, set] = useState(() => ({
    ...emptyContent(),
    ...entry?.content,
    title: entry?.content.title
      ? duplicate
        ? `${entry.content.title} — kopia`
        : entry.content.title
      : '',
    shortcut: duplicate ? '' : entry?.content.shortcut || '',
  }));
  const [tags, setTags] = useState(content.tags.join(', '));
  const [pinned, setPinned] = useState(entry?.pinned || false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [dirty, setDirty] = useState(false);
  const [discard, setDiscard] = useState(false);
  const [revisions, setRevisions] = useState<{ id: string; saved: number }[] | null>(null);
  const edit = (key: keyof typeof content, value: string) => {
    set((c) => ({ ...c, [key]: value }));
    setDirty(true);
  };
  const close = () => {
    if (busy) return;
    if (dirty) setDiscard(true);
    else onClose();
  };
  return (
    <Modal
      title={entry?.library && !duplicate ? 'Edytuj snippet' : 'Nowy snippet'}
      onClose={close}
      wide
    >
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          setBusy(true);
          setError('');
          try {
            const saved = await api.save(
              entry?.library && !duplicate ? entry.id : null,
              {
                ...content,
                tags: tags
                  .split(',')
                  .map((s) => s.trim())
                  .filter(Boolean),
              },
              pinned,
            );
            onSaved(saved);
          } catch (e) {
            setError(errorText(e));
          } finally {
            setBusy(false);
          }
        }}
      >
        <div className="modal-body">
          <p className="muted">Zapisz raz. Wykorzystuj wtedy, kiedy potrzebujesz.</p>
          {entry?.library && !duplicate ? (
            <div className="revision-picker">
              <button
                type="button"
                disabled={busy}
                onClick={async () => {
                  try {
                    setRevisions(await api.revisions(entry.id));
                  } catch (e) {
                    setError(errorText(e));
                  }
                }}
              >
                Poprzednie wersje
              </button>
              {revisions?.length === 0 ? (
                <p className="muted">Nie ma jeszcze zapisanych poprzednich wersji.</p>
              ) : null}
              {revisions?.map((r, i) => (
                <button
                  type="button"
                  key={r.id}
                  disabled={busy}
                  onClick={async () => {
                    setBusy(true);
                    try {
                      const c = await api.revision(entry.id, r.id);
                      set({ ...c, shortcut: content.shortcut });
                      setTags(c.tags.join(', '));
                      setDirty(true);
                      setRevisions(null);
                    } catch (e) {
                      setError(errorText(e));
                    } finally {
                      setBusy(false);
                    }
                  }}
                >
                  Wczytaj wersję {time(r.saved)} · {i + 1}
                </button>
              ))}
              {revisions?.length ? (
                <small>
                  Wczytanie zmienia formularz. Dopiero „Zapisz snippet” zatwierdza przywróconą
                  treść.
                </small>
              ) : null}
            </div>
          ) : null}
          <label>
            Nazwa
            <input
              autoFocus
              required
              maxLength={150}
              value={content.title}
              onChange={(e) => edit('title', e.target.value)}
              placeholder="np. Przegląd kodu przed publikacją"
            />
          </label>
          <div className="field-row">
            <label>
              Kategoria
              <input
                maxLength={50}
                value={content.category}
                onChange={(e) => edit('category', e.target.value)}
                placeholder="np. Praca z AI"
              />
            </label>
            <label>
              Tagi <span className="muted">· oddziel przecinkami</span>
              <input
                value={tags}
                onChange={(e) => {
                  setTags(e.target.value);
                  setDirty(true);
                }}
                placeholder="review, kod"
              />
            </label>
          </div>
          <label>
            Opis <span className="muted">· opcjonalnie</span>
            <input
              maxLength={2000}
              value={content.description}
              onChange={(e) => edit('description', e.target.value)}
            />
          </label>
          <label>
            Treść
            <textarea
              className="snippet-content"
              required
              value={content.text}
              onChange={(e) => edit('text', e.target.value)}
              spellCheck={false}
              placeholder="Twój prompt, wiadomość, kod lub checklista…"
            />
          </label>
          <div className="field-row">
            <label>
              Globalny skrót <span className="muted">· opcjonalnie</span>
              <input
                value={content.shortcut}
                onChange={(e) => edit('shortcut', e.target.value)}
                placeholder="Ctrl+Alt+1"
              />
              <small>
                {content.isTemplate
                  ? 'Otwiera formularz przed wklejeniem.'
                  : 'Wkleja bezpośrednio do aktywnej aplikacji.'}
              </small>
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={pinned}
                onChange={(e) => {
                  setPinned(e.target.checked);
                  setDirty(true);
                }}
              />
              <Pin size={16} />
              Dodaj do ulubionych
            </label>
          </div>
          <label className="check">
            <input
              type="checkbox"
              checked={content.isTemplate}
              onChange={(e) => {
                set((c) => ({ ...c, isTemplate: e.target.checked }));
                setDirty(true);
              }}
            />
            Szablon z polami do uzupełnienia
          </label>
          {content.isTemplate ? (
            <p className="muted">
              W treści użyj np. {'{{projekt}}'} lub {'{{cel}}'}. Powtórzone pole uzupełniasz raz.
              Żadne polecenia ani kod nie są wykonywane.
            </p>
          ) : null}
          {error ? (
            <p className="error-box" role="alert">
              {error}
            </p>
          ) : null}
          {discard ? (
            <div className="error-box">
              Porzucić niezapisane zmiany?
              <div className="button-row">
                <button type="button" onClick={() => setDiscard(false)}>
                  Wróć do edycji
                </button>
                <button type="button" className="danger" onClick={onClose}>
                  Porzuć zmiany
                </button>
              </div>
            </div>
          ) : null}
        </div>
        <footer>
          <span className="muted">{content.text.length.toLocaleString('pl-PL')} znaków</span>
          <div className="button-row">
            <button type="button" onClick={close}>
              Anuluj
            </button>
            <button className="primary" disabled={busy}>
              <Save size={16} />
              {busy ? 'Zapisywanie…' : 'Zapisz snippet'}
            </button>
          </div>
        </footer>
      </form>
    </Modal>
  );
}
