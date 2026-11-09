export function utc(time: string) {
  return new Intl.DateTimeFormat('ru-RU', {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: 'UTC',
  }).format(new Date(time)) + ' UTC';
}

export function kp(value: number) {
  return value.toLocaleString('ru-RU', { maximumFractionDigits: 2 });
}

