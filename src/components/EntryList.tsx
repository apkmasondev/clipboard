import { Pin, ArrowDownToLine, Search, ClipboardList } from 'lucide-react';
import type { Summary } from '../types';
import { kinds } from '../types';
import { size, time } from '../api';
import { KindIcon } from './Icons';
import { ImagePreview } from './ImagePreview';
export function EntryList({
  items,
  selected,
  onSelect,
  onActivate,
  loading,
  query,
  total,
  onMore,
  quick = false,
}: {
  items: Summary[];
  selected: string | null;
  onSelect: (id: string) => void;
  onActivate: (id: string) => void;
  loading: boolean;
  query: string;
  total: number;
  onMore: () => void;
  quick?: boolean;
}) {
  return (
    <div className="list-scroll" aria-busy={loading}>
      {!items.length && !loading ? (
        <div className="empty">
          <div className="empty-symbol">
            {query ? <Search size={30} /> : <ClipboardList size={30} />}
          </div>
          <h3>{query ? 'Brak pasujących elementów' : 'Miejsce na to, co kopiujesz'}</h3>
          <p>
            {query
              ? 'Spróbuj krótszej frazy lub zmień filtr.'
              : 'Skopiuj tekst, link lub obraz w dowolnej aplikacji. Pojawi się tutaj automatycznie.'}
          </p>
        </div>
      ) : null}
      {loading && !items.length ? (
        <p className="loading" role="status">
          Wczytywanie elementów…
        </p>
      ) : null}
      <div id="entries" role="listbox" aria-label="Elementy schowka" className="entries">
        {items.map((item) => (
          <button
            key={item.id}
            role="option"
            aria-selected={selected === item.id}
            tabIndex={selected === item.id ? 0 : -1}
            className={`entry ${selected === item.id ? 'selected' : ''}`}
            data-id={item.id}
            id={`entry-${item.id}`}
            onClick={() => onSelect(item.id)}
            onDoubleClick={() => onActivate(item.id)}
          >
            {item.kind === 'image' ? (
              <ImagePreview id={item.id} title={item.title} />
            ) : (
              <KindIcon kind={item.kind} />
            )}
            <span className="entry-body">
              <span className="entry-top">
                <strong>{item.title || 'Bez tytułu'}</strong>
                {item.pinned ? <Pin size={13} aria-label="Przypięty" /> : null}
              </span>
              <span className="entry-preview">
                {item.kind === 'image'
                  ? `${item.width && item.height ? `${item.width} × ${item.height} px · ` : ''}${size(item.bytes)}`
                  : item.preview || kinds[item.kind]}
              </span>
              <span className="entry-meta">
                <span>
                  {item.library ? (
                    <span className="snippet-label">
                      {item.isTemplate ? 'Szablon' : 'Snippet'}
                      {item.category ? ` / ${item.category}` : ''}
                    </span>
                  ) : (
                    kinds[item.kind]
                  )}
                </span>
                <span>{time(item.updated)}</span>
              </span>
            </span>
            {quick && selected === item.id ? <span className="enter-hint">↵</span> : null}
          </button>
        ))}
      </div>
      {items.length < total ? (
        <button className="load-more" disabled={loading} onClick={onMore}>
          <ArrowDownToLine size={14} />
          Wczytaj kolejne ({items.length} z {total})
        </button>
      ) : null}
    </div>
  );
}
