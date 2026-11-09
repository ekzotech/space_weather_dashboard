import { useCallback, useEffect, useState } from 'react';

export type Sample = { time: string; value: number };
export type Storm = { id: string; start: string; peak_kp: number; level: number; linked_events: string[] };
export type Aurora = {
  probability: number | null;
  observation_time: string;
  forecast_time: string;
  stale: boolean;
};

export async function get<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch('/api/' + path, { signal });
  if (!response.ok) throw new Error('HTTP ' + response.status);
  return response.json();
}

type Resource<T> = { status: 'loading' } | { status: 'error' } | { status: 'ready'; data: T };

export function useFeed<T>(path: string, interval?: number) {
  const [state, setState] = useState<Resource<T>>({ status: 'loading' });
  const [attempt, setAttempt] = useState(0);
  const retry = useCallback(() => {
    setState({ status: 'loading' });
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
        const data = await get<T>(path, controller.signal);
        if (!controller.signal.aborted) setState({ status: 'ready', data });
      } catch {
        if (!controller.signal.aborted) setState({ status: 'error' });
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


