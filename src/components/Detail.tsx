import { useEffect, useState } from 'react';
import {
  Copy,
  Pin,
  Trash2,
  Pencil,
  CopyPlus,
  BookmarkPlus,
  RotateCcw,
  WandSparkles,
  Tag,
  Check,
  Clipboard,
} from 'lucide-react';
import type { Entry } from '../types';
import { kinds } from '../types';
import { api, errorText, size, time, shortcutLabel } from '../api';
import { transforms, transform, type Transform } from '../transforms';
import { KindIcon } from './Icons';
export function Detail({
  entry,
  onCopy,
  onPin,
  onDelete,
  onEdit,
  onTags,
  notify,
}: {
  entry: Entry | null;
  onCopy: (id: string, text?: string) => void;
  onPin: (id: string) => void;
  onDelete: (e: Entry) => void;
  onEdit: (e: Entry, duplicate?: boolean) => void;
  onTags: () => void;
  notify: (s: string, error?: boolean) => void;
}) {
  const [image, setImage] = useState('');
  const [preview, setPreview] = useState<string | null>(null);
  const [operation, setOperation] = useState('');
  const [tags, setTags] = useState('');
  const [editingTags, setEditingTags] = useState(false);
  const [imageError, setImageError] = useState('');
  useEffect(() => {
    let active = true;
    setPreview(null);
    setOperation('');
    setImage('');
    setImageError('');
    setEditingTags(false);
    setTags(entry?.content.tags.join(', ') || '');
    if (entry?.kind === 'image')
      api
        .image(entry.id)
        .then((s) => {
          if (active) setImage(s);
        })
        .catch((e) => {
          if (active) setImageError(errorText(e));
        });
    return () => {
      active = false;
    };
  }, [entry?.id, entry?.content.text]);
  useEffect(() => {
    setTags(entry?.content.tags.join(', ') || '');
  }, [entry?.content.tags.join(',')]);
  if (!entry)
    return (
      <section className="detail blank">
        <Clipboard size={38} strokeWidth={1} />
        <h3>Wszystko pod ręką</h3>
        <p>
          Wybierz element, aby zobaczyć jego treść,
          <br />
          metadane i dostępne działania.
        </p>
        <div className="shortcut-note">
          <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>V</kbd>
          <span>Szybkie wklejanie w dowolnej aplikacji</span>
        </div>
      </section>
    );
  const c = entry.content;
  const text = preview ?? c.text;
  const isCode = ['code', 'json'].includes(entry.kind);
  return (
    <section className="detail">
      <header className="detail-toolbar">
        <span className="eyebrow">PODGLĄD</span>
        <div className="button-row">
          <button
            className={`icon-button ${entry.pinned ? 'active' : ''}`}
            title={entry.pinned ? 'Odepnij' : 'Przypnij do ulubionych'}
            aria-label={entry.pinned ? 'Odepnij' : 'Przypnij'}
            onClick={() => onPin(entry.id)}
          >
            <Pin size={17} />
          </button>
          {entry.library ? (
            <button
              className="icon-button"
              title="Edytuj snippet"
              aria-label="Edytuj snippet"
              onClick={() => onEdit(entry)}
            >
              <Pencil size={17} />
            </button>
          ) : null}
          <button
            className="icon-button delete"
            title="Usuń element"
            aria-label="Usuń element"
            onClick={() => onDelete(entry)}
          >
            <Trash2 size={17} />
          </button>
        </div>
      </header>
      <div className="detail-scroll">
        <div className="detail-title">
          <KindIcon kind={entry.kind} />
          <div>
            <span className="eyebrow">
              {entry.library ? 'BIBLIOTEKA SNIPPETÓW' : kinds[entry.kind].toLocaleUpperCase('pl')}
            </span>
            <h2>{c.title}</h2>
          </div>
        </div>
        {c.description ? <p className="description">{c.description}</p> : null}
        <div className="content-label">
          <span>{preview !== null ? 'WYNIK TRANSFORMACJI' : 'TREŚĆ'}</span>
          {preview !== null ? (
            <button
              className="text-button"
              onClick={() => {
                setPreview(null);
                setOperation('');
              }}
            >
              <RotateCcw size={13} />
              Oryginał
            </button>
          ) : null}
        </div>
        {entry.kind === 'image' ? (
          <div className="image-preview">
            {image ? (
              <img src={image} alt={c.title} />
            ) : (
              <p role="status">{imageError || 'Wczytywanie obrazu…'}</p>
            )}
          </div>
        ) : (
          <pre className={`content-preview ${isCode ? 'code' : ''}`}>{text.slice(0, 100000)}</pre>
        )}
        {text.length > 100000 ? (
          <p className="muted">Podgląd pierwszych 100 000 znaków. Kopiowana jest pełna treść.</p>
        ) : null}
        <div className="primary-actions">
          <button className="primary" onClick={() => onCopy(entry.id, preview ?? undefined)}>
            <Copy size={16} />
            {preview !== null ? 'Kopiuj wynik' : 'Kopiuj'}
            <kbd>Ctrl C</kbd>
          </button>
          <button
            title={entry.library ? 'Duplikuj snippet' : 'Zapisz jako snippet'}
            onClick={() =>
              onEdit(
                preview !== null
                  ? {
                      ...entry,
                      library: false,
                      content: { ...c, text: preview, html: [], rtf: [] },
                    }
                  : entry,
                entry.library,
              )
            }
          >
            {entry.library ? <CopyPlus size={16} /> : <BookmarkPlus size={16} />}
            <span>{entry.library ? 'Duplikuj' : 'Zapisz snippet'}</span>
          </button>
        </div>
        {entry.kind !== 'image' && entry.kind !== 'files' ? (
          <section className="detail-section">
            <h3>
              <WandSparkles size={15} />
              Transformacje tekstu
            </h3>
            <select
              aria-label="Wybierz transformację"
              value={operation}
              onChange={(e) => {
                const op = e.target.value;
                setOperation(op);
                if (op)
                  try {
                    setPreview(transform(c.text, op as Transform));
                  } catch {
                    notify(
                      'Nie można przekształcić tej treści. Sprawdź poprawność danych wejściowych.',
                      true,
                    );
                    setOperation('');
                    setPreview(null);
                  }
              }}
            >
              <option value="">Wybierz operację…</option>
              {Object.entries(transforms).map(([key, label]) => (
                <option key={key} value={key}>
                  {label}
                </option>
              ))}
            </select>
            <p className="help">
              Wynik sprawdzisz przed skopiowaniem. Oryginał pozostaje w historii.
            </p>
          </section>
        ) : null}
        <section className="detail-section">
          <h3>
            <Tag size={15} />
            Tagi
            <button className="text-button" onClick={() => setEditingTags(!editingTags)}>
              {editingTags ? 'Anuluj' : 'Edytuj'}
            </button>
          </h3>
          {editingTags ? (
            <form
              className="tag-form"
              onSubmit={async (e) => {
                e.preventDefault();
                try {
                  await api.tags(
                    entry.id,
                    tags
                      .split(',')
                      .map((t) => t.trim())
                      .filter(Boolean),
                  );
                  setEditingTags(false);
                  onTags();
                } catch (e) {
                  notify(errorText(e), true);
                }
              }}
            >
              <input
                aria-label="Tagi oddzielone przecinkami"
                value={tags}
                onChange={(e) => setTags(e.target.value)}
              />
              <button aria-label="Zapisz tagi">
                <Check size={16} />
              </button>
            </form>
          ) : (
            <div className="tags">
              {c.tags.length ? (
                c.tags.map((tag, i) => (
                  <span className="tag" key={`${tag}-${i}`}>
                    {tag}
                  </span>
                ))
              ) : (
                <span className="muted">Bez tagów</span>
              )}
            </div>
          )}
        </section>
        <section className="detail-section">
          <h3>Informacje</h3>
          <dl>
            <dt>Typ</dt>
            <dd>{kinds[entry.kind]}</dd>
            <dt>{entry.library ? 'Utworzono' : 'Pierwsza kopia'}</dt>
            <dd>{time(entry.created)}</dd>
            <dt>{entry.library ? 'Zmieniono' : 'Ostatnia kopia'}</dt>
            <dd>{time(entry.updated)}</dd>
            <dt>Źródło</dt>
            <dd>{entry.library ? 'Twoja biblioteka' : c.source || 'Nieustalone przez Windows'}</dd>
            <dt>Rozmiar</dt>
            <dd>
              {size(entry.bytes)}
              {entry.kind !== 'image'
                ? ` · ${Array.from(c.text).length.toLocaleString('pl-PL')} znaków`
                : ` · ${c.width} × ${c.height}`}
            </dd>
            {c.category ? (
              <>
                <dt>Kategoria</dt>
                <dd>{c.category}</dd>
              </>
            ) : null}
            {c.shortcut ? (
              <>
                <dt>Skrót</dt>
                <dd>{shortcutLabel(c.shortcut)}</dd>
              </>
            ) : null}
            <dt>Przechowywanie</dt>
            <dd>
              {entry.library
                ? 'Trwały snippet'
                : entry.pinned
                  ? 'Przypięty · chroniony przed cleanup'
                  : 'Zgodnie z limitem historii'}
            </dd>
          </dl>
        </section>
      </div>
    </section>
  );
}
