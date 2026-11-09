import type { ReactNode } from 'react';
import type { useFeed } from './api';

export function FeedContent<T>({ feed, children }: {
  feed: ReturnType<typeof useFeed<T>>;
  children: (data: T) => ReactNode;
}) {
  if (feed.status === 'loading') return <p role="status" className="muted">Загружаем данные…</p>;
  if (feed.status === 'error') return (
    <div className="space-y-3">
      <p role="status" className="muted">Источник данных временно недоступен.</p>
      <button className="secondary" onClick={feed.retry}>Повторить загрузку</button>
    </div>
  );
  return children(feed.data);
}

