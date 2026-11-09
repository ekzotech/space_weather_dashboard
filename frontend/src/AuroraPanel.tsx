import { useState, type FormEvent } from 'react';
import { get, type Aurora } from './api';
import { utc } from './format';

const cities = [
  { name: 'Мурманск', lat: 68.97, lon: 33.08 },
  { name: 'Екатеринбург', lat: 56.84, lon: 60.61 },
  { name: 'Архангельск', lat: 64.54, lon: 40.52 },
  { name: 'Санкт-Петербург', lat: 59.94, lon: 30.31 },
  { name: 'Москва', lat: 55.75, lon: 37.62 },
  { name: 'Тромсё', lat: 69.65, lon: 18.96 },
];

type Result =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; place: string; data: Aurora };

export function AuroraPanel() {
  const [city, setCity] = useState('0');
  const [result, setResult] = useState<Result>();
  const custom = city === 'custom';

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const point = custom
      ? { name: 'Выбранные координаты', lat: Number(form.get('lat')), lon: Number(form.get('lon')) }
      : cities[Number(city)];
    setResult({ status: 'loading' });
    try {
      const query = new URLSearchParams({ lat: String(point.lat), lon: String(point.lon) });
      const data = await get<Aurora>('aurora?' + query);
      setResult({ status: 'ready', place: point.name, data });
    } catch {
      setResult({ status: 'error' });
    }
  }

  return (
    <section className="panel aurora" id="aurora" aria-labelledby="aurora-title">
      <p className="eyebrow">03 / Локальный прогноз</p>
      <h2 id="aurora-title">Северное сияние</h2>
      <p className="muted mt-2">Проверьте оценку модели NOAA OVATION для вашей точки.</p>
      <form className="mt-6 grid gap-3" onSubmit={submit}>
        <fieldset className="grid gap-3" disabled={result?.status === 'loading'}>
          <label className="grid gap-1 text-sm">
            Город
            <select value={city} onChange={event => { setCity(event.target.value); setResult(undefined); }}>
              {cities.map((item, index) => <option key={item.name} value={index}>{item.name}</option>)}
              <option value="custom">Свои координаты</option>
            </select>
          </label>
          {custom && (
            <div className="grid gap-3 min-[360px]:grid-cols-2">
              <label className="grid gap-1 text-sm">Широта<input name="lat" type="number" min="-90" max="90" step="any" required /></label>
              <label className="grid gap-1 text-sm">Долгота<input name="lon" type="number" min="-180" max="180" step="any" required /></label>
            </div>
          )}
          <button className="primary" type="submit">Проверить мой город</button>
        </fieldset>
      </form>
      <div className="result mt-5" aria-live="polite">
        {!result && <p className="muted">Оценка появится после проверки.</p>}
        {result?.status === 'loading' && <p>Проверяем прогноз…</p>}
        {result?.status === 'error' && <p>Не удалось получить прогноз. Попробуйте ещё раз.</p>}
        {result?.status === 'ready' && (
          <>
            <p className="eyebrow">{result.place}</p>
            {result.data.stale
              ? <p>Последний прогноз устарел.</p>
              : result.data.probability === null
                ? <p>Для этой точки нет данных.</p>
                : <p className="metric mt-1">{result.data.probability}%</p>}
            <p className="muted mt-2 text-sm">Время прогноза · {utc(result.data.forecast_time)}</p>
          </>
        )}
      </div>
      <p className="muted mt-4 text-sm">Модель оценивает вероятность сияния над точкой. Наблюдение также зависит от темноты, облачности и засветки.</p>
    </section>
  );
}

