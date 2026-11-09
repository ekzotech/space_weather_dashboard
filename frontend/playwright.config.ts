import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL: 'http://127.0.0.1:4173',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 4173 --strictPort',
    url: 'http://127.0.0.1:4173',
  },
  projects: [
    { name: 'desktop', use: devices['Desktop Chrome'] },
    { name: 'mobile', use: devices['Pixel 7'] },
    {
      name: 'firefox',
      use: devices['Desktop Firefox'],
      testMatch: 'dashboard.spec.ts',
      grep: /renders both charts and shows an upcoming storm/,
    },
    {
      name: 'webkit',
      use: devices['Desktop Safari'],
      testMatch: 'dashboard.spec.ts',
      grep: /renders both charts and shows an upcoming storm/,
    },
  ],
});



