import { useEffect, useState, type FormEvent } from 'react';
import { useFeed, type Storm } from './api';
import { FeedContent } from './FeedContent';
import { kp, utc } from './format';

type Sort = 'peak' | 'date';
type Range = { start: string; end: string };

function StormList({ minimum, range, sort }: { minimum: number; range?: Range; sort: Sort }) {
  const query = new URLSearchParams({ min_kp: String(minimum), ...range });
  const feed = useFeed<Storm[]>('storms?' + query);
  return (
    <FeedContent feed={feed}>
      {storms => storms.length ? <StormPages key={sort} storms={storms} sort={sort} /> : <p className="muted">{range ? 'За выбранный период бурь с таким Kp не найдено.' : 'За 30 дней бурь с таким Kp не найдено.'}</p>}
    </FeedContent>
  );
}

const PAGE_SIZE = 5;

function StormPages({ storms, sort }: { storms: Storm[]; sort: Sort }) {
  const [page, setPage] = useState(0);
  const ordered = sort === 'date' ? [...storms].sort((a, b) => Date.parse(b.start) - Date.parse(a.start)) : storms;
  const pageCount = Math.ceil(ordered.length / PAGE_SIZE);
  const current = Math.min(page, pageCount - 1);
  const visible = ordered.slice(current * PAGE_SIZE, (current + 1) * PAGE_SIZE);
  return (
    <>
        <ul className="divide-y divide-[#eaeaea]">
          {visible.map(storm => (
            <li key={storm.id} className="py-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
              <time dateTime={storm.start}>{utc(storm.start)}</time>
              <span className="mono text-sm">Kp {kp(storm.peak_kp)} · G{storm.level}</span>
              </div>
              <details className="mt-2 text-sm">
                <summary className="cursor-pointer">Связанные события</summary>
                <p className="muted mt-2">Источник пикового Kp · {storm.peak_source || 'не указан'}</p>
                {storm.linked_events.length
                  ? <ul className="mt-2 break-all">{storm.linked_events.map(id => <li key={id}>{id}</li>)}</ul>
                  : <p className="muted mt-2">Связи не указаны.</p>}
              </details>
            </li>
          ))}
        </ul>
      {pageCount > 1 && <nav className="mt-3 flex flex-wrap items-center gap-2 text-sm" aria-label="Страницы архива">
        <button className="secondary" disabled={current === 0} onClick={() => setPage(current - 1)}>Назад</button>
        <span aria-live="polite">{current + 1} из {pageCount}</span>
        <button className="secondary" disabled={current + 1 === pageCount} onClick={() => setPage(current + 1)}>Вперёд</button>
      </nav>}
    </>
  );
}

const EMPTY_RANGE = { start: '', end: '' };

function validRange(start: string, end: string, today: string) {
  const validDate = (date: string) => {
    const parsed = Date.parse(date);
    return /^\d{4}-\d{2}-\d{2}$/.test(date) && Number.isFinite(parsed) &&
      new Date(parsed).toISOString().slice(0, 10) === date;
  };
  return validDate(start) && validDate(end) && start <= end && end <= today &&
    Date.parse(end) - Date.parse(start) < 30 * 86_400_000;
}

function restoreFilters(today: string) {
  const query = new URLSearchParams(window.location.search);
  const start = query.get('storm_start') ?? '';
  const end = query.get('storm_end') ?? '';
  const min = Number(query.get('storm_kp'));
  return {
    minimum: Number.isInteger(min) && min >= 5 && min <= 9 ? min : 5,
    sort: (query.get('storm_sort') === 'date' ? 'date' : 'peak') as Sort,
    range: validRange(start, end, today) ? { start, end } : undefined,
  };
}

export function StormArchive() {
  const today = new Date().toISOString().slice(0, 10);
  const [initial] = useState(() => restoreFilters(today));
  const [minimum, setMinimum] = useState(initial.minimum);
  const [sort, setSort] = useState(initial.sort);
  const [range, setRange] = useState(initial.range);
  const [draft, setDraft] = useState(initial.range ?? EMPTY_RANGE);
  const [rangeError, setRangeError] = useState(false);

  useEffect(() => {
    const url = new URL(window.location.href);
    for (const [name, value] of Object.entries({
      storm_kp: minimum === 5 ? undefined : String(minimum),
      storm_sort: sort === 'peak' ? undefined : sort,
      storm_start: range?.start,
      storm_end: range?.end,
    })) {
      if (value === undefined) url.searchParams.delete(name);
      else url.searchParams.set(name, value);
    }
    window.history.replaceState(window.history.state, '', url);
  }, [minimum, sort, range]);

  function applyRange(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const start = String(data.get('start'));
    const end = String(data.get('end'));
    if (!validRange(start, end, today)) {
      setRangeError(true);
      return;
    }
    setRangeError(false);
    setRange({ start, end });
  }
  return (
    <section className="panel" aria-labelledby="archive-title">
      <p className="eyebrow">04 / NASA DONKI</p>
      <h2 id="archive-title">Архив бурь</h2>
      <p className="muted mt-2">{range ? 'Выбранный период' : 'Последние 30 дней'}.</p>
      <form onSubmit={applyRange} className="mt-4 grid gap-2">
        <div className="grid gap-2 min-[420px]:grid-cols-2">
          <label className="grid gap-1 text-sm">Начало периода<input name="start" type="date" max={today} value={draft.start} onChange={event => setDraft({ ...draft, start: event.target.value })} required /></label>
          <label className="grid gap-1 text-sm">Конец периода<input name="end" type="date" max={today} value={draft.end} onChange={event => setDraft({ ...draft, end: event.target.value })} required /></label>
        </div>
        <div className="flex flex-wrap gap-2">
          <button type="submit" className="secondary">Применить даты</button>
          {range && <button type="button" className="secondary" onClick={() => { setRange(undefined); setDraft(EMPTY_RANGE); setRangeError(false); }}>Сбросить даты</button>}
        </div>
        {rangeError && <p role="alert" className="muted text-sm">Укажите период до 30 календарных дней в UTC.</p>}
      </form>
      <label className="my-4 grid max-w-48 gap-1 text-sm">
        Минимальный Kp
        <select value={minimum} onChange={event => setMinimum(Number(event.target.value))}>
          {[5, 6, 7, 8, 9].map(level => <option key={level} value={level}>{level}</option>)}
        </select>
      </label>
      <label className="my-4 grid max-w-48 gap-1 text-sm" htmlFor="archive-sort">
        Сортировка архива
        <select id="archive-sort" value={sort} onChange={event => setSort(event.target.value as Sort)}>
          <option value="peak">По пиковому Kp</option>
          <option value="date">По дате</option>
        </select>
      </label>
      <StormList sort={sort} key={JSON.stringify([minimum, range])} minimum={minimum} range={range} />
    </section>
  );
}



