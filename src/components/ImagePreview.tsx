import { useCallback, useEffect, useRef, useState } from 'react';
import { Image as ImageIcon, ImageOff } from 'lucide-react';
import { api } from '../api';
import { useEvent } from '../hooks';
import { PreviewQueue } from '../previewQueue';

const previews = new PreviewQueue(api.thumbnail);

export function usePreviewCacheLifecycle() {
  useEvent(
    'changed',
    useCallback(() => previews.clear(), []),
  );
  useEffect(() => {
    const clearHidden = () => {
      if (document.hidden) previews.clear();
    };
    document.addEventListener('visibilitychange', clearHidden);
    return () => {
      document.removeEventListener('visibilitychange', clearHidden);
      previews.clear();
    };
  }, []);
}

export function ImagePreview({
  id,
  title,
  large = false,
}: {
  id: string;
  title: string;
  large?: boolean;
}) {
  const container = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(false);
  const [result, setResult] = useState<{ id: string; src: string; error: boolean } | null>(null);
  useEffect(() => {
    let intersects = false;
    const update = () => setVisible(intersects && !document.hidden);
    const observer = new IntersectionObserver(([entry]) => {
      intersects = entry.isIntersecting;
      update();
    });
    if (container.current) observer.observe(container.current);
    document.addEventListener('visibilitychange', update);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', update);
    };
  }, []);
  useEffect(() => {
    setResult(null);
    if (!visible) return;
    const controller = new AbortController();
    previews
      .request(id, controller.signal)
      .then((src) => {
        if (!controller.signal.aborted) setResult({ id, src, error: false });
      })
      .catch(() => {
        if (!controller.signal.aborted) setResult({ id, src: '', error: true });
      });
    return () => controller.abort();
  }, [id, visible]);
  const current = result?.id === id ? result : null;
  return (
    <span
      ref={container}
      className={`image-thumbnail ${large ? 'large' : ''}`}
      aria-busy={visible && !current}
    >
      {current?.src && !current.error ? (
        <img
          src={current.src}
          alt={large ? title : ''}
          decoding="async"
          onError={() => setResult({ id, src: '', error: true })}
        />
      ) : (
        <span
          className="thumbnail-placeholder"
          role={current?.error ? 'img' : undefined}
          aria-label={current?.error ? 'Podgląd niedostępny' : undefined}
        >
          {current?.error ? (
            <ImageOff size={large ? 28 : 20} aria-hidden="true" />
          ) : (
            <ImageIcon size={large ? 28 : 20} aria-hidden="true" />
          )}
          {large ? (
            <span>{current?.error ? 'Podgląd niedostępny' : 'Wczytywanie podglądu…'}</span>
          ) : null}
        </span>
      )}
    </span>
  );
}
