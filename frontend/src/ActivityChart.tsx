import { useEffect, useRef } from 'react';
import { Chart, type Plugin, LineController, LineElement, PointElement, LinearScale, LogarithmicScale, Tooltip } from 'chart.js';
import type { Sample } from './api';
import { utc } from './format';

const stormThreshold: Plugin<'line'> = {
  id: 'stormThreshold',
  afterDatasetsDraw({ ctx, chartArea, scales }) {
    const y = scales.y.getPixelForValue(5);
    ctx.save();
    ctx.strokeStyle = '#956400';
    ctx.setLineDash([5, 4]);
    ctx.beginPath();
    ctx.moveTo(chartArea.left, y);
    ctx.lineTo(chartArea.right, y);
    ctx.stroke();
    ctx.restore();
  },
};

Chart.register(LineController, LineElement, PointElement, LinearScale, LogarithmicScale, Tooltip);

function chartPoints(points: Sample[], expectedStep: number) {
  const data: { x: number; y: number | null }[] = [];
  for (const point of points) {
    const x = Date.parse(point.time);
    const previous = data.at(-1);
    if (previous && x - previous.x > expectedStep * 1.5) {
      data.push({ x: previous.x + expectedStep, y: null });
    }
    data.push({ x, y: point.value });
  }
  return data;
}

export function ActivityChart({ points, title, solar = false }: {
  points: Sample[];
  title: string;
  solar?: boolean;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvas.current || !points.length) return;
    const chart = new Chart(canvas.current, {
      type: 'line',
      plugins: solar ? [] : [stormThreshold],
      data: {
        datasets: [{
          data: chartPoints(points, solar ? 60_000 : 10_800_000),
          borderColor: solar ? '#95553c' : '#34604f',
          borderWidth: 2,
          pointRadius: points.length === 1 ? 3 : 0,
          pointHoverRadius: 4,
          tension: 0,
        }],
      },
      options: {
        animation: false,
        maintainAspectRatio: false,
        interaction: { mode: 'nearest', axis: 'x', intersect: false },
        plugins: { tooltip: { callbacks: { title: items => utc(new Date(items[0].parsed.x ?? 0).toISOString()) } } },
        scales: {
          x: {
            type: 'linear',
            grid: { display: false },
            ticks: {
              maxTicksLimit: 5,
              color: '#5c625a',
              callback: value => solar
                ? new Date(Number(value)).toISOString().slice(11, 16)
                : new Date(Number(value)).toISOString().slice(5, 10),
            },
          },
          y: solar ? {
            type: 'logarithmic',
            grid: { color: '#eaeaea' },
            ticks: { color: '#5c625a' },
          } : {
            type: 'linear',
            min: 0,
            max: 9,
            grid: { color: '#eaeaea' },
            ticks: { color: '#5c625a', stepSize: 3 },
          },
        },
      },
    });
    canvas.current.dataset.points = String(points.length);
    return () => chart.destroy();
  }, [points, solar]);

  if (!points.length) return <p className="muted">За этот период данных нет.</p>;
  return (
    <figure>
      <div className="relative h-48 sm:h-56">
        <canvas ref={canvas} role="img" aria-label={title} />
      </div>
      <figcaption className="muted mt-3 text-sm">
        {points.length} измерений · последнее {utc(points[points.length - 1].time)}
      </figcaption>
      {!solar && <p className="muted mt-1 text-xs">Пунктир · порог бури Kp 5</p>}
      <details className="mt-3 text-sm">
        <summary className="cursor-pointer">Последние измерения</summary>
        <table className="mt-2 w-full text-left">
          <thead><tr><th>Время</th><th>{solar ? 'Вт/м²' : 'Kp'}</th></tr></thead>
          <tbody>
            {points.slice(-8).reverse().map(point => (
              <tr key={point.time}><td>{utc(point.time)}</td><td>{solar ? point.value.toExponential(2) : point.value}</td></tr>
            ))}
          </tbody>
        </table>
      </details>
    </figure>
  );
}


