import type { Sample } from './api';

export function downloadSeries(points: Sample[], filename: string, column: string) {
  const csv = ['time_utc,' + column, ...points.map(point => point.time + ',' + point.value)].join('\r\n') + '\r\n';
  const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv;charset=utf-8' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

