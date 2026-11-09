import { createContext, useContext } from 'react';

export type Zone = 'utc' | 'device';
export const ChartTime = createContext<Zone>('utc');
export const useChartTime = () => useContext(ChartTime);

export function chartTime(time: string, zone: Zone) {
  return new Intl.DateTimeFormat('ru-RU', {
    dateStyle: 'medium', timeStyle: 'short', timeZone: zone === 'utc' ? 'UTC' : undefined,
  }).format(new Date(time)) + (zone === 'utc' ? ' UTC' : '');
}

