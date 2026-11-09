import { useState } from 'react';
import { ActivityChart } from './ActivityChart';
import { FeedContent } from './FeedContent';
import type { Sample, useFeed } from './api';

export function KpPanel({ observations }: { observations: ReturnType<typeof useFeed<Sample[]>> }) {
  const [days, setDays] = useState(7);
  return (
    <section className="panel min-w-0" aria-labelledby="kp-title">
      <p className="eyebrow">01 / NOAA SWPC</p>
      <h2 id="kp-title">Геомагнитная активность</h2>
      <label className="my-3 grid max-w-48 gap-1 text-sm" htmlFor="kp-period">
        Период Kp
        <select id="kp-period" value={days} onChange={event => setDays(Number(event.target.value))}>
          <option value="1">24 часа</option>
          <option value="3">3 дня</option>
          <option value="7">7 дней</option>
        </select>
      </label>
      <FeedContent feed={observations}>
        {points => {
          const end = Date.parse(points.at(-1)?.time ?? '');
          const range = points.filter(point => Date.parse(point.time) >= end - days * 86_400_000);
          return <ActivityChart points={range} title="График планетарного Kp" />;
        }}
      </FeedContent>
    </section>
  );
}

