import { useState } from 'react';
import { api, errorText } from '../api';
import { Modal } from './Modal';
export function BackupDialog({
  onClose,
  onDone,
}: {
  onClose: () => void;
  onDone: (text: string) => void;
}) {
  const [restore, setRestore] = useState(false);
  const [password, setPassword] = useState('');
  const [repeat, setRepeat] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  return (
    <Modal
      title="Kopia zapasowa biblioteki"
      onClose={() => {
        if (!busy) onClose();
      }}
    >
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (busy) return;
          setError('');
          if (!restore && password !== repeat) {
            setError('Hasła muszą być identyczne.');
            return;
          }
          setBusy(true);
          try {
            const n = await api.backup(password, restore);
            if (n !== null) {
              setPassword('');
              setRepeat('');
              onDone(
                restore
                  ? `Odtworzono nowych snippetów: ${n}. Istniejące pozostały bez zmian.`
                  : `Zapisano szyfrowaną kopię ${n} snippetów.`,
              );
            }
          } catch (e) {
            setError(errorText(e));
          } finally {
            setBusy(false);
          }
        }}
      >
        <div className="modal-body">
          <div className="segmented" aria-label="Działanie kopii">
            <button
              type="button"
              disabled={busy}
              aria-pressed={!restore}
              onClick={() => {
                setRestore(false);
                setError('');
              }}
            >
              Zapisz kopię
            </button>
            <button
              type="button"
              disabled={busy}
              aria-pressed={restore}
              onClick={() => {
                setRestore(true);
                setError('');
              }}
            >
              Odtwórz kopię
            </button>
          </div>
          <p>
            {restore
              ? 'Kopia doda brakujące snippety, kategorie, tagi i ulubione. Nie nadpisze obecnych danych. Skróty zostaną wyłączone, aby uniknąć konfliktów.'
              : 'Kopia obejmuje snippety, ich pola, kategorie, tagi i ulubione. Możesz przenieść ją na inne konto lub komputer.'}
          </p>
          <p className="muted">
            Historia schowka, ustawienia i poprzednie wersje snippetów nie są częścią tej kopii.
          </p>
          <label>
            Hasło kopii — minimum 12 znaków
            <input
              type="password"
              required
              minLength={12}
              maxLength={256}
              autoComplete={restore ? 'current-password' : 'new-password'}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={busy}
            />
          </label>
          {!restore ? (
            <label>
              Powtórz hasło
              <input
                type="password"
                required
                minLength={12}
                maxLength={256}
                autoComplete="new-password"
                value={repeat}
                onChange={(e) => setRepeat(e.target.value)}
                disabled={busy}
              />
            </label>
          ) : null}
          <p className="muted">
            Hasło nie jest zapisywane. Zachowaj je — bez niego nie odzyskasz kopii. Szyfrowanie
            odbywa się lokalnie.
          </p>
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
          <button className="primary" disabled={busy}>
            {busy ? 'Przetwarzanie…' : restore ? 'Wybierz kopię' : 'Zaszyfruj i zapisz'}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
