import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { Settings } from './types';
export function useEvent<T>(name: string, handler: (payload: T) => void) {
  const [failure, setFailure] = useState<Error | null>(null);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<T>(name, (e) => handler(e.payload))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        if (!disposed) setFailure(new Error('Nie można połączyć zdarzeń aplikacji.'));
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [name, handler]);
  if (failure) throw failure;
}
export function useDebounce<T>(value: T, delay = 120) {
  const [v, set] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => set(value), delay);
    return () => clearTimeout(t);
  }, [value, delay]);
  return v;
}
export function useTheme(theme: Settings['theme'] | undefined) {
  useEffect(() => {
    const mq = matchMedia('(prefers-color-scheme: dark)');
    const update = () => {
      document.documentElement.dataset.theme =
        theme === 'system' || !theme ? (mq.matches ? 'dark' : 'light') : theme;
    };
    update();
    mq.addEventListener('change', update);
    return () => mq.removeEventListener('change', update);
  }, [theme]);
}
