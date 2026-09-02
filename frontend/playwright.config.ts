// End-to-end browser tests against the running docker stack
// (`docker compose up` serves the whole app on :5100). Set E2E_BASE_URL
// to point somewhere else, e.g. a vite dev server proxying a local API.
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:5100',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
});
