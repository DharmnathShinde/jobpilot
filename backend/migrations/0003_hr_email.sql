-- HR contact + outreach tracking on jobs; raise default discovery volume.

ALTER TABLE jobs
    ADD COLUMN IF NOT EXISTS hr_email TEXT,
    ADD COLUMN IF NOT EXISTS hr_email_source TEXT,
    ADD COLUMN IF NOT EXISTS outreach_status TEXT NOT NULL DEFAULT 'none';

ALTER TABLE jobs DROP CONSTRAINT IF EXISTS jobs_outreach_status_check;
ALTER TABLE jobs
    ADD CONSTRAINT jobs_outreach_status_check
    CHECK (outreach_status IN ('none', 'drafted', 'sent'));

ALTER TABLE jobs DROP CONSTRAINT IF EXISTS jobs_hr_email_source_check;
ALTER TABLE jobs
    ADD CONSTRAINT jobs_hr_email_source_check
    CHECK (hr_email_source IS NULL OR hr_email_source IN ('jd', 'manual'));

ALTER TABLE preferences
    ALTER COLUMN max_jobs_per_run SET DEFAULT 200;

-- Bump legacy personal defaults so morning runs aren't stuck at 20.
UPDATE preferences SET max_jobs_per_run = 200 WHERE max_jobs_per_run <= 20;
