import { useEffect, useState } from 'react';
import { api, errorText } from '../api';
import type { Entry } from '../types';
import { Modal } from './Modal';
import { getCurrentWindow } from '@tauri-apps/api/window';

export function TemplateComposer({
  entry,
  paste,
  onClose,
  onDone,
}: {
  entry: Entry;
  paste: boolean;
  onClose: () => void;
  onDone: () => void;
}) {
  const [fields, setFields] = useState<string[]>([]);
  const [values, setValues] = useState<Record<string, string>>({});
  const [preview, setPreview] = useState<string | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    api
      .templateFields(entry.id)
      .then((f) => {
        if (active) {
          setFields(f);
          setValues(Object.fromEntries(f.map((n) => [n, ''])));
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
  }, [entry.id]);
  useEffect(() => {
    setPreview(null);
    if (loading || fields.some((n) => !values[n]?.trim())) return;
    let active = true;
    const timer = setTimeout(() => {
      api
        .renderTemplate(entry.id, values)
        .then((p) => {
          if (active) {
            setPreview(p);
            setError('');
          }
        })
        .catch((e) => {
          if (active) setError(errorText(e));
        });
    }, 120);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [entry.id, values, fields, loading]);
  return (
    <Modal
      title={entry.content.title}
      onClose={() => {
        if (!busy) onClose();
      }}
    >
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (busy || preview === null) return;
          setBusy(true);
          setError('');
          try {
            const text = await api.renderTemplate(entry.id, values);
            await api.copy(entry.id, text, paste);
            onDone();
          } catch (e) {
            setError(errorText(e));
            if (paste) {
              await getCurrentWindow().show();
              await getCurrentWindow().setFocus();
            }
          } finally {
            setBusy(false);
          }
        }}
      >
        <div className="modal-body template-form">
          <p className="muted">
            Uzupełnij pola. Wartości są używane tylko w tej kopii i nie zmieniają snippetu.
          </p>
          {loading ? (
            <p role="status">Wczytywanie pól…</p>
          ) : (
            fields.map((name, i) => (
              <label key={name}>
                {name}
                <textarea
                  autoFocus={i === 0}
                  rows={2}
                  required
                  maxLength={100000}
                  value={values[name] ?? ''}
                  onChange={(e) => setValues((v) => ({ ...v, [name]: e.target.value }))}
                />
              </label>
            ))
          )}
          <details>
            <summary>Podgląd gotowego tekstu</summary>
            <pre className="template-preview">
              {preview?.slice(0, 100000) ?? 'Uzupełnij wszystkie pola, aby zobaczyć wynik.'}
            </pre>
          </details>
          {error ? (
            <p className="error-box" role="alert">
              {error}
            </p>
          ) : null}
        </div>
        <footer>
          <button type="button" disabled={busy} onClick={onClose}>
            Anuluj
          </button>
          <button className="primary" disabled={busy || preview === null}>
            {busy ? 'Proszę czekać…' : paste ? 'Wklej gotowy tekst' : 'Kopiuj gotowy tekst'}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
