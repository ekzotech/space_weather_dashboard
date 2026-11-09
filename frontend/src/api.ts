import { useCallback, useEffect, useState } from 'react';

export type Sample = { time: string; value: number };
export type Storm = { id: string; start: string; peak_kp: number; level: number; linked_events: string[] };
export type Aurora = {
  probability: number | null;
  grid_distance_km: number | null;
  observation_time: string;
  forecast_time: string;
  stale: boolean;
};

async function fetchFeed<T>(path: string, signal?: AbortSignal) {
  const response = await fetch('/api/' + path, { signal });
  if (!response.ok) throw new Error('HTTP ' + response.status);
  return {
    data: await response.json() as T,
    receivedAt: response.headers.get("x-feed-received-at") ?? new Date().toISOString(),
    stale: response.headers.get("x-feed-stale") === "true",
  };
}

export async function get<T>(path: string, signal?: AbortSignal): Promise<T> {
  return (await fetchFeed<T>(path, signal)).data;
}

type Resource<T> = { status: 'loading' } | { status: 'error'; lastSuccess?: string } |
  { status: 'ready'; data: T; receivedAt: string; stale: boolean };

export function useFeed<T>(path: string, interval?: number) {
  const [state, setState] = useState<Resource<T>>({ status: 'loading' });
  const [attempt, setAttempt] = useState(0);
  const retry = useCallback(() => {
    // Keep the previous result available if the retry also fails.
    setAttempt(value => value + 1);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    let lastAttempt = 0;
    let inFlight = false;
    async function refresh() {
      if (inFlight) return;
      inFlight = true;
      lastAttempt = Date.now();
      try {
        const data = await fetchFeed<T>(path, controller.signal);
        if (!controller.signal.aborted) setState({ status: 'ready', ...data });
      } catch {
        if (!controller.signal.aborted) setState(previous => ({
          status: 'error',
          lastSuccess: previous.status === 'ready' ? previous.receivedAt :
            previous.status === 'error' ? previous.lastSuccess : undefined,
        }));
      } finally {
        inFlight = false;
      }
    }
    void refresh();
    const refreshVisible = () => {
      if (interval && document.visibilityState === 'visible' && Date.now() - lastAttempt >= interval) {
        void refresh();
      }
    };
    const timer = interval && window.setInterval(refreshVisible, interval);
    document.addEventListener('visibilitychange', refreshVisible);
    return () => {
      controller.abort();
      document.removeEventListener('visibilitychange', refreshVisible);
      window.clearInterval(timer);
    };
  }, [path, interval, attempt]);

  return { ...state, retry };
}



