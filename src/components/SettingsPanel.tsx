import { useEffect, useState } from 'react';
import {
  ShieldCheck,
  Keyboard,
  HardDrive,
  Sun,
  Power,
  Save,
  Trash2,
  Pause,
  Play,
} from 'lucide-react';
import type { Settings, Stats, Kind } from '../types';
import { kinds } from '../types';
import { api, errorText, size } from '../api';
export function SettingsPanel({
  settings,
  stats,
  onSaved,
  onClear,
  onBackup,
  notify,
  onDirtyChange,
}: {
  settings: Settings;
  stats: Stats | null;
  onSaved: () => void;
  onClear: () => void;
  onBackup: () => void;
  notify: (message: string, error?: boolean) => void;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const [draft, set] = useState(settings);
  const [apps, setApps] = useState(settings.blockedApps.join('\n'));
  const [busy, setBusy] = useState(false);
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    onDirtyChange(dirty);
    return () => onDirtyChange(false);
  }, [dirty, onDirtyChange]);
  const update = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    set((s) => ({ ...s, [key]: value }));
    setDirty(true);
  };
  return (
    <form
      className="settings-panel"
      onSubmit={async (e) => {
        e.preventDefault();
        setBusy(true);
        try {
          await api.saveSettings({
            ...draft,
            blockedApps: [
              ...new Set(
                apps
                  .split(/[\n,;]/)
                  .map((s) => s.trim().toLowerCase())
                  .filter(Boolean),
              ),
            ],
          });
          setDirty(false);
          notify('Ustawienia zapisane.');
          onSaved();
        } catch (e) {
          notify(errorText(e), true);
        } finally {
          setBusy(false);
        }
      }}
    >
      <header className="settings-heading">
        <div>
          <span className="eyebrow">TWÓJ SPOSÓB PRACY</span>
          <h1>Ustawienia</h1>
          <p>Prywatność, skróty i porządek w historii.</p>
        </div>
        <button className="primary" disabled={busy || !dirty}>
          <Save size={16} />
          {busy ? 'Zapisywanie…' : 'Zapisz zmiany'}
        </button>
      </header>
      <div className="settings-scroll">
        <section className="settings-section">
          <h2>
            <HardDrive size={19} />
            Kopia biblioteki
          </h2>
          <p className="section-intro">
            Zabezpiecz snippety plikiem szyfrowanym hasłem. Przenieś bibliotekę na inny komputer lub
            odtwórz brakujące wpisy.
          </p>
          <button type="button" onClick={onBackup}>
            Zapisz lub odtwórz kopię
          </button>
        </section>
        <section className="settings-section">
          <h2>
            <ShieldCheck size={19} />
            Prywatność
          </h2>
          <p className="section-intro">
            Treści zostają na tym komputerze. Zapisane dane są szyfrowane przez Windows DPAPI dla
            Twojego konta.
          </p>
          <label className="setting-line">
            <span>
              <strong>{draft.paused ? 'Historia wstrzymana' : 'Automatyczne zapisywanie'}</strong>
              <small>Wyłączenie nie usuwa zapisanych elementów. Snippety nadal działają.</small>
            </span>
            <span className="toggle-label">
              {draft.paused ? <Pause size={15} /> : <Play size={15} />}
              <input
                type="checkbox"
                role="switch"
                checked={!draft.paused}
                onChange={(e) => update('paused', !e.target.checked)}
              />
            </span>
          </label>
          <label className="setting-line">
            <span>
              <strong>Pomijaj rozpoznawalne sekrety</strong>
              <small>
                Klucze prywatne, tokeny z jednoznacznym prefiksem i JWT. Nie wykrywa każdego hasła.
              </small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={draft.skipSecrets}
              onChange={(e) => update('skipSecrets', e.target.checked)}
            />
          </label>
          <label className="setting-line">
            <span>
              <strong>Pomijaj nieznane aplikacje źródłowe</strong>
              <small>Więcej prywatności. Część aplikacji nie ujawnia właściciela schowka.</small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={draft.blockUnknown}
              onChange={(e) => update('blockUnknown', e.target.checked)}
            />
          </label>
          <label>
            Wykluczone aplikacje
            <textarea
              className="apps-input"
              value={apps}
              onChange={(e) => {
                setApps(e.target.value);
                setDirty(true);
              }}
              spellCheck={false}
            />
            <small>
              Jedna nazwa procesu .exe w wierszu, np. keepassxc.exe. Wykluczenie przeglądarki
              dotyczy wszystkich jej kart; nie rozpoznajemy stron bankowych.
            </small>
          </label>
          <div className="field-label">Zapisywane typy danych</div>
          <div className="type-checks">
            {Object.entries(kinds).map(([key, label]) => (
              <label className="check" key={key}>
                <input
                  type="checkbox"
                  checked={!draft.disabledTypes.includes(key as Kind)}
                  onChange={(e) =>
                    update(
                      'disabledTypes',
                      e.target.checked
                        ? draft.disabledTypes.filter((t) => t !== key)
                        : [...draft.disabledTypes, key as Kind],
                    )
                  }
                />
                {label}
              </label>
            ))}
          </div>
        </section>
        <section className="settings-section">
          <h2>
            <HardDrive size={19} />
            Historia i miejsce na dysku
          </h2>
          <div className="limit-fields">
            <label>
              Liczba elementów
              <input
                type="number"
                required
                min={10}
                max={20000}
                value={draft.maxItems}
                onChange={(e) => update('maxItems', e.target.valueAsNumber)}
              />
            </label>
            <label>
              Maksymalny wiek · dni
              <input
                type="number"
                required
                min={1}
                max={3650}
                value={draft.maxDays}
                onChange={(e) => update('maxDays', e.target.valueAsNumber)}
              />
            </label>
            <label>
              Limit historii · MB
              <input
                type="number"
                required
                min={10}
                max={2048}
                value={draft.maxMb}
                onChange={(e) => update('maxMb', e.target.valueAsNumber)}
              />
            </label>
          </div>
          <p className="help">
            Limity dotyczą nieprzypiętej historii. Snippety i przypięte wpisy są trwałe. Rozmiar
            plików może obejmować wolne strony SQLite oraz narzut szyfrowania. Zmniejszenie limitów
            usuwa najstarsze wpisy po zapisaniu ustawień.
          </p>
          <div className="storage-bar">
            <span>
              {stats
                ? `${stats.history} elementów historii · ${stats.snippets} snippetów`
                : 'Obliczanie…'}
            </span>
            <strong>{stats ? size(stats.diskBytes) : '—'} na dysku</strong>
          </div>
          <button type="button" className="danger-subtle" onClick={onClear}>
            <Trash2 size={15} />
            Wyczyść nieprzypiętą historię
          </button>
        </section>
        <section className="settings-section">
          <h2>
            <Keyboard size={19} />
            Szybki dostęp
          </h2>
          <label>
            Skrót okna wklejania
            <input
              value={draft.quickShortcut}
              required
              onChange={(e) => update('quickShortcut', e.target.value)}
              placeholder="Ctrl+Shift+V"
            />
            <small>
              Przykłady: Ctrl+Shift+V, Ctrl+Alt+Space. Konflikt zostanie sprawdzony przy zapisie.
            </small>
          </label>
          <p className="help">
            W oknie szybkiego wyboru: ↑ ↓ wybór, Enter wklej, Esc zamknij. Skróty pojedynczych
            snippetów ustawisz w ich edytorze.
          </p>
        </section>
        <section className="settings-section">
          <h2>
            <Sun size={19} />
            Wygląd
          </h2>
          <div className="theme-options" role="group" aria-label="Motyw">
            {(['system', 'light', 'dark'] as const).map((t) => (
              <button
                type="button"
                aria-pressed={draft.theme === t}
                className={draft.theme === t ? 'selected' : ''}
                key={t}
                onClick={() => update('theme', t)}
              >
                {t === 'system' ? 'Jak w Windows' : t === 'light' ? 'Jasny' : 'Ciemny'}
              </button>
            ))}
          </div>
        </section>
        <section className="settings-section">
          <h2>
            <Power size={19} />
            Uruchamianie
          </h2>
          <label className="setting-line">
            <span>
              <strong>Startuj razem z Windows</strong>
              <small>
                Uruchomienie w trayu po zalogowaniu. Zamknięcie okna pozostawia historię aktywną.
              </small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={draft.autostart}
              onChange={(e) => update('autostart', e.target.checked)}
            />
          </label>
        </section>
        <p className="about">
          Super Clipboard 1.0.0 · Windows · lokalnie
          <br />
          Dane: %LOCALAPPDATA%\pl.superclipboard.desktop
          <br />
          DPAPI nie chroni przed programami działającymi na tym samym koncie. Eksport snippetów jest
          jawnym plikiem JSON.
        </p>
      </div>
    </form>
  );
}
