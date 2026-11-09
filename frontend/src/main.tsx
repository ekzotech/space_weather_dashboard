import { StrictMode, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ActivityChart } from './ActivityChart';
import { useFeed, type Sample } from './api';
import { ChartTime, type Zone } from './ChartTime';
import { AuroraPanel } from './AuroraPanel';
import { downloadSeries } from './csv';
import { FeedContent } from './FeedContent';
import { KpPanel } from './KpPanel';
import { Overview } from './Overview';
import { StormArchive } from './StormArchive';
import './styles.css';

function Dashboard() {
  const [zone, setZone] = useState<Zone>('utc');
  const observations = useFeed<Sample[]>('kp', 300_000);
  const xray = useFeed<Sample[]>('xray', 60_000);

  return (
    <div className="shell mx-auto max-w-6xl px-4 sm:px-8">
      <header className="masthead flex flex-wrap items-center justify-between gap-x-6 gap-y-3 py-5">
        <div className="flex items-center gap-3">
          <svg width="28" height="28" viewBox="0 0 28 28" fill="none" aria-hidden="true">
            <circle cx="14" cy="14" r="5" fill="currentColor" />
            <ellipse cx="14" cy="14" rx="13" ry="7" stroke="currentColor" transform="rotate(-34 14 14)" />
          </svg>
          <span className="eyebrow">Space Weather Dashboard</span>
        </div>
        <a className="text-sm underline underline-offset-4" href="#aurora">Проверить сияние</a>
      </header>
      <main className="pb-12">
        <section className="hero">
          <p className="eyebrow">Мониторинг Земли и Солнца</p>
          <h1 className="mt-3">Космическая<br className="sm:hidden" /> погода</h1>
          <p className="muted mt-4 max-w-lg">Солнечная активность, магнитные бури и прогноз северного сияния для вашего города.</p>
        </section>
        <Overview observations={observations} />
        <label className="mt-5 grid max-w-56 gap-1 text-sm" htmlFor="chart-time">
          Время графиков
          <select id="chart-time" value={zone} onChange={event => setZone(event.target.value as Zone)}>
            <option value="utc">UTC</option>
            <option value="device">Время устройства</option>
          </select>
        </label>
        <ChartTime.Provider value={zone}>
        <div className="charts mt-4 grid gap-4 lg:grid-cols-[1.25fr_1fr]">
          <KpPanel observations={observations} />
          <section className="panel min-w-0" aria-labelledby="solar-title">
            <p className="eyebrow">02 / NOAA GOES</p>
            <h2 id="solar-title">Солнечная активность</h2>
            <p className="muted mb-4 mt-1 text-sm">Поток рентгеновского излучения · 24 часа · Вт/м²</p>
            <FeedContent feed={xray}>
              {points => (
                <>
                  <ActivityChart points={points} title="График солнечного рентгеновского потока" solar />
                  {!!points.length && <button className="secondary mt-3" onClick={() => downloadSeries(points, 'goes-xray.csv', 'watts_per_square_metre')}>Скачать GOES CSV</button>}
                </>
              )}
            </FeedContent>
          </section>
        </div>
        </ChartTime.Provider>
        <div className="mt-4 grid items-start gap-4 lg:grid-cols-[1fr_1.25fr]">
          <AuroraPanel />
          <StormArchive />
        </div>
      </main>
      <footer className="masthead flex flex-wrap justify-between gap-3 py-5 text-sm muted">
        <span>Время предупреждений, прогноза сияния и архива в UTC</span>
        <div className="flex gap-4">
          <a href="https://www.swpc.noaa.gov/">NOAA SWPC</a>
          <a href="https://ccmc.gsfc.nasa.gov/tools/DONKI/">NASA DONKI</a>
        </div>
      </footer>
    </div>
  );
}

createRoot(document.getElementById('root')!).render(<StrictMode><Dashboard /></StrictMode>);


