# Deploy JobPilot for free (personal use)

Total cost: ₹0. Takes about 30 minutes the first time.

| Piece | Service | Free limits that matter |
| --- | --- | --- |
| Database | [Neon](https://neon.com) | 1 GB storage, sleeps after 5 min idle, no card needed |
| API (Rust) | [Render](https://render.com) web service | Sleeps after 15 min idle; ~1 min to wake; 750 hours/month |
| Morning run | GitHub Actions (this repo) | Wakes the API hourly and runs your morning job |
| Phone app | Expo EAS Build → Android APK | 15 Android builds/month |
| Web app (optional, iPhone) | Render static site | Free |

## 1. Put the code on GitHub

Create a **private** repo, then:

```bash
cd jobpilot
git remote add origin https://github.com/<you>/jobpilot.git
git push -u origin main
```

## 2. Database: Neon

1. Sign up at neon.com, create a project, region **AWS Asia Pacific (Singapore)**.
2. Copy the connection string (it looks like `postgresql://user:pass@ep-xxx.ap-southeast-1.aws.neon.tech/neondb?sslmode=require`).

The API creates all tables itself on first start.

## 3. API: Render

1. Generate a secret for the morning job: `openssl rand -hex 32` (or any 40+ random characters). Keep it.
2. On render.com: **New → Blueprint**, pick your repo. Render reads `render.yaml`.
3. When it asks for values:
   - `DATABASE_URL` → the Neon string
   - `CRON_SECRET` → the secret from step 1
   - `ANTHROPIC_API_KEY` → optional, for smarter CV reading (leave blank to skip)
4. Wait for the first deploy (Rust build takes ~10 minutes the first time).
5. Open `https://<your-api>.onrender.com/health`. It should say `{"ok":true}`.

If Render gives the API a different URL than `jobpilot-api.onrender.com`, update it in `app/eas.json` and in the `jobpilot-web` section of `render.yaml`.

## 4. Morning run: GitHub Actions

Render's free plan sleeps, so the built-in scheduler can't wake itself. In your GitHub repo:
**Settings → Secrets and variables → Actions → New repository secret**

- `API_URL` = `https://<your-api>.onrender.com`
- `CRON_SECRET` = the same secret as on Render

Then **Actions → Morning run → Run workflow** once to check it goes green. After that it runs hourly
and does your job search when your chosen time (default 7:00 IST, weekdays) has passed.

## 5. Phone app: Android APK

```bash
npm install -g eas-cli
cd app
eas login                 # free Expo account
eas build:configure       # accept defaults; links the project
eas build -p android --profile apk
```

When the build finishes (10–20 min on the free queue), open the link on your phone, download the APK
and install it (allow "install unknown apps" for your browser when asked).

**iPhone:** installing your own build needs a paid Apple developer account. Use the web app instead:
open the `jobpilot-web` Render URL in Safari → Share → **Add to Home Screen**.

## 6. Lock it to just you

1. Open the app and create your account.
2. On Render → `jobpilot-api` → Environment, set `ALLOW_SIGNUPS` to `false`. Render redeploys.

Nobody else can now create an account on your server.

## Good to know

- **First tap of the day is slow** (~1 minute) while Render wakes the API. After that it's fast.
- **Data is safe while asleep**: everything lives in Neon, not on Render.
- **Updating**: push to `main` and Render redeploys the API. Rebuild the APK only when app code changes.
- **Backups**: Neon keeps a short restore history on the free plan; for your own copy run
  `pg_dump "<neon url>" > backup.sql` now and then.
