import { expect, test } from '@playwright/test';

const first = '2026-10-10T00:00:00Z';
const second = '2026-10-10T03:00:00Z';
const peak = { id: 'example', start: second, peak_kp: 6.33, level: 2, linked_events: [] };

test.beforeEach(async ({ page }) => {
  await page.route('**/api/kp', route => route.fulfill({
    json: [{ time: first, value: 3 }, { time: second, value: 6.33 }],
  }));
  await page.route('**/api/forecast', route => route.fulfill({ json: [{ time: second, value: 7 }] }));
  await page.route('**/api/xray', route => route.fulfill({
    json: [{ time: first, value: 1e-7 }, { time: second, value: 2e-6 }],
  }));
  await page.route('**/api/storms?*', route => {
    const min = Number(new URL(route.request().url()).searchParams.get('min_kp'));
    return route.fulfill({ json: min <= peak.peak_kp ? [peak] : [] });
  });
});

test('renders both charts and shows an upcoming storm', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Космическая');
  for (const name of ['График планетарного Kp', 'График солнечного рентгеновского потока']) {
    const chart = page.getByRole('img', { name, exact: true });
    await expect(chart).toBeVisible();
    await expect(chart).toHaveAttribute('data-points', '2');
  }
  await expect(page.getByText('Ожидается G3')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize()!.width);
});

test('checks the selected city and displays the model result', async ({ page }) => {
  await page.route('**/api/aurora?*', route => {
    const { searchParams } = new URL(route.request().url());
    expect(searchParams.get('lat')).toBe('56.84');
    expect(searchParams.get('lon')).toBe('60.61');
    return route.fulfill({
      json: { probability: 24, observation_time: first, forecast_time: second, stale: false },
    });
  });

  await page.goto('/');
  await page.getByLabel('Город', { exact: true }).selectOption({ label: 'Екатеринбург' });
  await page.getByRole('button', { name: 'Проверить мой город' }).click();
  await expect(page.getByText('24%', { exact: true })).toBeVisible();
  await expect(page.getByText('Время прогноза')).toBeVisible();
});

test('filters storms by minimum Kp', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText('Kp 6,33 · G2')).toBeVisible();
  await page.getByLabel('Минимальный Kp').selectOption('7');
  await expect(page.getByText('За 30 дней бурь с таким Kp не найдено.')).toBeVisible();
});

test('recovers when the solar feed is temporarily unavailable', async ({ page }) => {
  const fail = '**/api/xray';
  const unavailable = (route: import('@playwright/test').Route) => route.fulfill({ status: 502 });
  await page.route(fail, unavailable);
  await page.goto('/');
  const solar = page.getByRole('region', { name: 'Солнечная активность' });
  await expect(solar.getByText('Источник данных временно недоступен.')).toBeVisible();
  await page.unroute(fail, unavailable);
  await solar.getByRole('button', { name: 'Повторить загрузку' }).click();
  await expect(solar.getByRole('img')).toHaveAttribute('data-points', '2');
});

