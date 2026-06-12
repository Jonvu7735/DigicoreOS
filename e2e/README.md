# Frontend smoke tests (e2e)

Browser smoke tests for the DigicoreOS web UI, driven with Playwright. They are
**self-contained**: each test seeds a demo session into `localStorage` and stubs
every `/api/v1/*` call with canned JSON, so **no backend services are required**.
Playwright builds the SPA and serves it with Vite's preview server automatically.

What they cover: the login screen renders, and the loyalty and shipments screens
render their data against the typed contract (catching regressions in routing,
auth gating, and the page-shaped list responses).

## Run

```bash
cd e2e
npm install
npm run install-browser   # downloads Chromium for Playwright
npm test
```

To use a Chromium that's already on disk (e.g. an offline runner), point at it:

```bash
PW_EXECUTABLE_PATH=/path/to/chrome npm test
```

> Not wired into CI yet (it would need a browser download in the pipeline). It's
> a committed, repeatable local smoke check; promoting it to CI is a follow-up.
