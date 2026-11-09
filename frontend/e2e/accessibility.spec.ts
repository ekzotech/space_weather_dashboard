import { expect, test } from '@playwright/test';

const first = '2026-10-10T00:00:00Z';
const second = '2026-10-10T03:00:00Z';

test.beforeEach(async ({ page }) => {
  const points = [{ time: first, value: 5 }, { time: second, value: 7 }];
  await page.route('**/api/kp', route => route.fulfill({ json: points }));
  await page.route('**/api/forecast', route => route.fulfill({ json: points }));
  await page.route('**/api/xray', route => route.fulfill({ json: [{ time: first, value: 2e-6 }] }));
  await page.route('**/api/storms?*', route => route.fulfill({ json: [] }));
  await page.route('**/api/aurora?*', route => route.fulfill({
    json: { probability: 24, observation_time: first, forecast_time: second, stale: false },
  }));
  await page.route('**/api/cloud?*', route => route.fulfill({ json: { cloud_cover: null, time: '2026-10-10T00:00:00Z' } }));
});

test('semantic warning and muted text meet AA contrast', async ({ page }) => {
  await page.goto('/');
  const warning = page.getByText('Ожидается G3');
  await expect(warning).toBeVisible();
  for (const locator of [warning, page.getByText('Мониторинг Земли и Солнца')]) {
    const contrast = await locator.evaluate(node => {
      const channels = (color: string) => color.match(/[\d.]+/g)!.map(Number);
      const luminance = (color: number[]) => color.slice(0, 3).map(channel => {
        const rgb = channel / 255;
        return rgb <= 0.04045 ? rgb / 12.92 : ((rgb + 0.055) / 1.055) ** 2.4;
      }).reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);

      let background = [255, 255, 255];
      for (let element: Element | null = node; element; element = element.parentElement) {
        const color = channels(getComputedStyle(element).backgroundColor);
        if (color[3] === undefined || color[3] === 1) {
          background = color;
          break;
        }
      }
      const foreground = luminance(channels(getComputedStyle(node).color));
      const back = luminance(background);
      return (Math.max(foreground, back) + 0.05) / (Math.min(foreground, back) + 0.05);
    });
    expect(contrast).toBeGreaterThanOrEqual(4.5);
  }
});

test('opens an aurora estimate with the keyboard only', async ({ page }) => {
  await page.goto('/');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Перейти к содержимому' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('main')).toBeFocused();

  const check = page.getByRole('button', { name: 'Проверить мой город' });
  for (let index = 0; index < 30 && !await check.evaluate(node => node === document.activeElement); index++) {
    await page.keyboard.press('Tab');
  }
  await expect(check).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByText('24%', { exact: true })).toBeVisible();
});

test('fits a 320px screen with both chart tables expanded', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 720 });
  await page.goto('/');
  await expect(page.getByText('Последние измерения')).toHaveCount(2);
  for (const summary of await page.getByText('Последние измерения').all()) await summary.click();
  await expect(page.getByRole('table')).toHaveCount(2);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
});

test('preserves the aurora form at 200% zoom', async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 760 });
  await page.goto('/');
  await page.evaluate(() => { document.documentElement.style.zoom = '2'; });
  const button = page.getByRole('button', { name: 'Проверить мой город' });
  await expect(button).toBeVisible();
  const bounds = await button.boundingBox();
  expect(bounds!.x).toBeGreaterThanOrEqual(0);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(640);
  await button.click();
  await expect(page.getByText('24%', { exact: true })).toBeVisible();
});

test('shows a linked inline error for out-of-range coordinates', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('combobox', { name: 'Город', exact: true }).selectOption('custom');
  const latitude = page.locator('input[name="lat"]');
  await latitude.fill('91');
  await page.getByLabel('Долгота').fill('0');
  await page.getByRole('button', { name: 'Проверить мой город' }).click();
  await expect(latitude).toHaveAttribute('aria-invalid', 'true');
  await expect(latitude).toHaveAccessibleDescription('Введите число от -90 до 90.');
  await latitude.fill('69');
  await expect(latitude).toHaveAttribute('aria-invalid', 'false');
});

test('exposes the changing aurora result as an accessible status', async ({ page }) => {
  await page.goto('/');
  const aurora = page.getByRole('region', { name: 'Северное сияние' });
  const status = aurora.getByRole('status');
  expect(await status.ariaSnapshot()).toContain('Оценка появится после проверки.');
  await aurora.getByRole('button', { name: 'Проверить мой город' }).click();
  await expect(status).toContainText('24%');
  expect(await status.ariaSnapshot()).toContain('24%');
  await expect(status).toHaveAttribute('aria-live', 'polite');
  await expect(status).toHaveAttribute('aria-atomic', 'true');
});

test('keeps the forecast warning readable in forced colors', async ({ page }) => {
  await page.emulateMedia({ forcedColors: 'active' });
  await page.goto('/');
  const warning = page.getByText('Ожидается G3');
  await expect(warning).toBeVisible();
  await expect(warning).toHaveCSS('border-top-width', '1px');
  await expect(warning).toHaveCSS('border-top-style', 'solid');
});


