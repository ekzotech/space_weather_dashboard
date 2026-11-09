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
  { name: 'Киров', region: 'Кировская область', lat: 58.60, lon: 49.67 },
  { name: 'Киров', region: 'Калужская область', lat: 54.08, lon: 34.30 },
  { name: 'Берёзовский', region: 'Свердловская область', lat: 56.91, lon: 60.80 },
  { name: 'Берёзовский', region: 'Кемеровская область', lat: 55.67, lon: 86.27 },
];

function placeLabel(place: { name: string; region?: string }) {
  return place.region ? place.name + ' · ' + place.region : place.name;
}

function normalizeName(text: string) {
  return text.trim().toLocaleLowerCase('ru').replaceAll('ё', 'е');
}

type Result =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; place: string; data: Aurora };

const initialPlace = { city: '0', coordinates: { lat: '', lon: '' } };

function savedPlace(): typeof initialPlace {
  try {
    const saved = JSON.parse(window.localStorage.getItem('aurora-place') ?? 'null');
    if (saved?.city === 'custom' && typeof saved.coordinates?.lat === 'string' && typeof saved.coordinates?.lon === 'string') {
      const { lat, lon } = saved.coordinates;
      if (lat.trim() && lon.trim() && Math.abs(Number(lat)) <= 90 && Math.abs(Number(lon)) <= 180) {
        return { city: 'custom', coordinates: { lat, lon } };
      }
    }
    if (typeof saved?.city === 'string' && cities.some((_, index) => String(index) === saved.city)) {
      return { ...initialPlace, city: saved.city };
    }
  } catch { /* storage unavailable */ }
  return initialPlace;
}

export function AuroraPanel() {
  const [initial] = useState(savedPlace);
  const [city, setCity] = useState(initial.city);
  const [search, setSearch] = useState('');
  const [coordinates, setCoordinates] = useState(initial.coordinates);
  const [locating, setLocating] = useState(false);
  const [locationError, setLocationError] = useState(false);
  const matching = cities.map((item, index) => ({ ...item, index })).filter(
    item => normalizeName(placeLabel(item)).includes(normalizeName(search)),
  );
  const selected = city === 'custom' || matching.some(item => String(item.index) === city)
    ? city : matching.length === 1 ? String(matching[0].index) : '';
  const [result, setResult] = useState<Result>();
  const custom = selected === 'custom';

  function locate() {
    const unavailable = () => {
      setCity('custom');
      setResult(undefined);
      setLocationError(true);
      setLocating(false);
    };
    setLocationError(false);
    if (!navigator.geolocation) {
      unavailable();
      return;
    }
    setLocating(true);
    navigator.geolocation.getCurrentPosition(position => {
      setCoordinates({
        lat: position.coords.latitude.toFixed(5),
        lon: position.coords.longitude.toFixed(5),
      });
      setCity('custom');
      setResult(undefined);
      setLocating(false);
    }, unavailable, { timeout: 10_000, maximumAge: 60_000 });
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const point = custom
      ? { name: 'Выбранные координаты', lat: Number(form.get('lat')), lon: Number(form.get('lon')) }
      : cities[Number(selected)];
    try {
      window.localStorage.setItem('aurora-place', JSON.stringify({
        city: selected,
        coordinates: custom ? coordinates : initialPlace.coordinates,
      }));
    } catch { /* storage unavailable */ }
    setResult({ status: 'loading' });
    try {
      const query = new URLSearchParams({ lat: String(point.lat), lon: String(point.lon) });
      const data = await get<Aurora>('aurora?' + query);
      setResult({ status: 'ready', place: placeLabel(point), data });
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
        <fieldset className="grid gap-3" disabled={result?.status === 'loading' || locating}>
          <label className="grid gap-1 text-sm" htmlFor="city-search">
            Поиск по каталогу городов
            <input id="city-search" type="search" value={search} onChange={event => {
              setSearch(event.target.value);
              setCity('');
              setResult(undefined);
            }} placeholder="Введите название" />
          </label>
          <div className="grid gap-1 text-sm">
            <label htmlFor="aurora-city">Город</label>
            <select id="aurora-city" value={selected} onChange={event => { setCity(event.target.value); setResult(undefined); }}>
              <option value="" disabled>Выберите город</option>
              {matching.map(item => <option key={item.index} value={item.index}>{placeLabel(item)}</option>)}
              <option value="custom">Свои координаты</option>
            </select>
          </div>
          {!matching.length && <p className="muted text-sm">Городов не найдено. Можно ввести свои координаты.</p>}
          <button type="button" className="secondary" onClick={locate}>
            {locating ? 'Определяем координаты…' : 'Использовать моё местоположение'}
          </button>
          {locationError && <p role="status" className="muted text-sm">Не удалось определить местоположение. Введите координаты вручную.</p>}
          {custom && (
            <div className="grid gap-3 min-[360px]:grid-cols-2">
              <label className="grid gap-1 text-sm">Широта<input name="lat" value={coordinates.lat} onChange={event => setCoordinates({ ...coordinates, lat: event.target.value })} type="number" min="-90" max="90" step="any" required /></label>
              <label className="grid gap-1 text-sm">Долгота<input name="lon" value={coordinates.lon} onChange={event => setCoordinates({ ...coordinates, lon: event.target.value })} type="number" min="-180" max="180" step="any" required /></label>
            </div>
          )}
          <button className="primary" type="submit" disabled={!selected}>Проверить мой город</button>
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
            <p className="muted mt-2 text-sm">Наблюдение модели · {utc(result.data.observation_time)}</p>
            <p className="muted mt-1 text-sm">Время прогноза · {utc(result.data.forecast_time)}</p>
          </>
        )}
      </div>
      <p className="muted mt-4 text-sm">Модель оценивает вероятность сияния над точкой. Наблюдение также зависит от темноты, облачности и засветки.</p>
    </section>
  );
}


