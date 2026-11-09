import { useEffect, useState } from 'react';
import { useFeed, type Sample } from './api';
import { FeedContent } from './FeedContent';
import { kp, utc } from './format';

import { stormWindows, WINDOW_MS } from './stormWindows';

function forecastEnd(time: string) {
  return utc(new Date(Date.parse(time) + WINDOW_MS).toISOString());
}

export function Overview({ observations }: { observations: ReturnType<typeof useFeed<Sample[]>> }) {
  const [threshold, setThreshold] = useState(() => {
    try {
      const saved = Number(window.localStorage.getItem('forecast-threshold'));
      return Number.isInteger(saved) && saved >= 5 && saved <= 9 ? saved : 5;
    } catch {
      return 5;
    }
  });
  useEffect(() => {
    try { window.localStorage.setItem('forecast-threshold', String(threshold)); } catch { /* storage unavailable */ }
  }, [threshold]);
  const forecast = useFeed<Sample[]>('forecast', 300_000);
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <section className="panel overview" aria-label="Текущий Kp">
        <p className="eyebrow">Наблюдаемый Kp</p>
        <FeedContent feed={observations}>
          {points => {
            const latest = points.at(-1);
            if (!latest) return <p className="muted">Нет измерений.</p>;
            const stale = Date.now() - Date.parse(latest.time) > 6 * 60 * 60_000;
            return (
              <>
                <p className="metric mt-2">{kp(latest.value)} <span className="muted text-lg">/ 9</span></p>
                <p className="muted mt-2 text-sm">{utc(latest.time)}{stale && ' · данные устарели'}</p>
                {!stale && latest.value >= 5 && <p className="badge warning mt-3">Буря G{Math.floor(latest.value) - 4}</p>}
              </>
            );
          }}
        </FeedContent>
      </section>
      <section className="panel overview" aria-label="Предупреждения о бурях">
        <p className="eyebrow">Предупреждения · NOAA</p>
        <label className="mt-3 grid max-w-48 gap-1 text-sm" htmlFor="forecast-threshold">
          Порог предупреждений
          <select id="forecast-threshold" value={threshold} onChange={event => setThreshold(Number(event.target.value))}>
            {[5, 6, 7, 8, 9].map(value => <option key={value} value={value}>Kp {value}</option>)}
          </select>
        </label>
        <FeedContent feed={forecast}>
          {points => {
            const windows = stormWindows(points, threshold);
            const next = windows[0];
            if (!points.length) return <p className="muted mt-2">Актуального прогноза пока нет.</p>;
            const through = <p className="muted mt-2 text-sm">Горизонт · до {forecastEnd(points.at(-1)!.time)}</p>;
            return next ? (
              <>
                <p className="badge warning mt-3">Ожидается G{Math.floor(next.peak) - 4}</p>
                <ul className="mt-2 divide-y divide-[#eaeaea]" aria-label="Окна бурь">
                  {windows.map(window => <li key={window.start} className="py-2 text-sm">
                    {utc(window.start)} — {utc(window.end)} · пик Kp {kp(window.peak)} · G{Math.floor(window.peak) - 4}
                  </li>)}
                </ul>
                {through}
              </>
            ) : (
              <>
                <p className="badge calm mt-3">Kp ниже {threshold}</p>
                <p className="muted mt-3 text-sm">На доступном горизонте Kp не достигает выбранного порога.</p>
                {through}
              </>
            );
          }}
        </FeedContent>
      </section>
    </div>
  );
}


