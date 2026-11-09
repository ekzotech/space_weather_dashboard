import { useFeed, type Sample } from './api';
import { FeedContent } from './FeedContent';
import { kp, utc } from './format';

export function Overview({ observations }: { observations: ReturnType<typeof useFeed<Sample[]>> }) {
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
        <FeedContent feed={forecast}>
          {points => {
            const next = points.find(point => point.value >= 5);
            if (!points.length) return <p className="muted mt-2">Актуального прогноза пока нет.</p>;
            return next ? (
              <>
                <p className="badge warning mt-3">Ожидается G{Math.floor(next.value) - 4}</p>
                <p className="mt-3">Kp {kp(next.value)} · {utc(next.time)}</p>
                <p className="muted mt-1 text-sm">Начало прогнозного трёхчасового окна.</p>
              </>
            ) : (
              <>
                <p className="badge calm mt-3">Kp ниже 5</p>
                <p className="muted mt-3 text-sm">На доступном горизонте прогноза бурь не ожидается.</p>
              </>
            );
          }}
        </FeedContent>
      </section>
    </div>
  );
}

