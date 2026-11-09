import { useState } from 'react';
import { useFeed, type Storm } from './api';
import { FeedContent } from './FeedContent';
import { kp, utc } from './format';

function StormList({ minimum }: { minimum: number }) {
  const feed = useFeed<Storm[]>('storms?min_kp=' + minimum);
  return (
    <FeedContent feed={feed}>
      {storms => storms.length ? (
        <ul className="divide-y divide-[#eaeaea]">
          {storms.map(storm => (
            <li key={storm.id} className="flex flex-wrap items-center justify-between gap-2 py-3">
              <time dateTime={storm.start}>{utc(storm.start)}</time>
              <span className="mono text-sm">Kp {kp(storm.peak_kp)} · G{storm.level}</span>
            </li>
          ))}
        </ul>
      ) : <p className="muted">За 30 дней бурь с таким Kp не найдено.</p>}
    </FeedContent>
  );
}

export function StormArchive() {
  const [minimum, setMinimum] = useState(5);
  return (
    <section className="panel" aria-labelledby="archive-title">
      <p className="eyebrow">04 / NASA DONKI</p>
      <h2 id="archive-title">Архив бурь</h2>
      <p className="muted mt-2">Последние 30 дней, по убыванию пикового Kp.</p>
      <label className="my-4 grid max-w-48 gap-1 text-sm">
        Минимальный Kp
        <select value={minimum} onChange={event => setMinimum(Number(event.target.value))}>
          {[5, 6, 7, 8, 9].map(level => <option key={level} value={level}>{level}</option>)}
        </select>
      </label>
      <StormList key={minimum} minimum={minimum} />
    </section>
  );
}

