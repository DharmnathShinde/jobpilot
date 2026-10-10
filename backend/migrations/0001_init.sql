-- JobPilot initial schema

CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The master profile: the only source of facts the app may use in resumes and drafts.
CREATE TABLE profiles (
    user_id          UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    full_name        TEXT NOT NULL DEFAULT '',
    headline         TEXT NOT NULL DEFAULT '',
    location         TEXT NOT NULL DEFAULT '',
    email            TEXT NOT NULL DEFAULT '',
    phone            TEXT NOT NULL DEFAULT '',
    summary          TEXT NOT NULL DEFAULT '',
    years_experience REAL NOT NULL DEFAULT 0,
    skills           TEXT[] NOT NULL DEFAULT '{}',
    experience       JSONB NOT NULL DEFAULT '[]',
    projects         JSONB NOT NULL DEFAULT '[]',
    education        JSONB NOT NULL DEFAULT '[]',
    links            JSONB NOT NULL DEFAULT '[]',
    raw_cv_text      TEXT NOT NULL DEFAULT '',
    confirmed        BOOLEAN NOT NULL DEFAULT false,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE preferences (
    user_id          UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    target_roles     TEXT[] NOT NULL DEFAULT '{}',
    locations        TEXT[] NOT NULL DEFAULT '{}',
    remote_ok        BOOLEAN NOT NULL DEFAULT true,
    expected_ctc_lpa REAL,
    notice_days      INT,
    min_score        INT NOT NULL DEFAULT 50,
    run_time         TIME NOT NULL DEFAULT '07:00',
    timezone         TEXT NOT NULL DEFAULT 'Asia/Kolkata',
    active_days      INT[] NOT NULL DEFAULT '{1,2,3,4,5}', -- ISO weekday, Mon=1
    max_jobs_per_run INT NOT NULL DEFAULT 20,
    schedule_enabled BOOLEAN NOT NULL DEFAULT true,
    last_run_on      DATE,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Public job boards the user follows (Greenhouse / Lever / Ashby company boards).
CREATE TABLE job_sources (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('greenhouse', 'lever', 'ashby')),
    board      TEXT NOT NULL,
    company    TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, kind, board)
);

CREATE TABLE jobs (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source           TEXT NOT NULL, -- manual | share | greenhouse | lever | ashby
    external_id      TEXT NOT NULL,
    title            TEXT NOT NULL,
    company          TEXT NOT NULL DEFAULT '',
    location         TEXT NOT NULL DEFAULT '',
    url              TEXT NOT NULL DEFAULT '',
    description      TEXT NOT NULL DEFAULT '',
    posted_at        TIMESTAMPTZ,
    required_skills  TEXT[] NOT NULL DEFAULT '{}',
    min_years        REAL,
    max_years        REAL,
    score            INT NOT NULL DEFAULT 0,
    matched_skills   TEXT[] NOT NULL DEFAULT '{}',
    missing_skills   TEXT[] NOT NULL DEFAULT '{}',
    explanation      TEXT NOT NULL DEFAULT '',
    queue_status     TEXT NOT NULL DEFAULT 'new' CHECK (queue_status IN ('new', 'saved', 'skipped', 'applied')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, source, external_id)
);
CREATE INDEX jobs_user_queue_idx ON jobs (user_id, queue_status, score DESC);

CREATE TABLE tailored_resumes (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id           UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    content          JSONB NOT NULL,
    changes          JSONB NOT NULL DEFAULT '[]',
    keyword_coverage INT NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE applications (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id        UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    status        TEXT NOT NULL CHECK (status IN ('saved', 'applied', 'replied', 'interview', 'offer', 'rejected')),
    resume_id     UUID REFERENCES tailored_resumes(id) ON DELETE SET NULL,
    notes         TEXT NOT NULL DEFAULT '',
    next_reminder DATE,
    applied_at    TIMESTAMPTZ,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, job_id)
);

CREATE TABLE application_events (
    id             UUID PRIMARY KEY,
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,
    note           TEXT NOT NULL DEFAULT '',
    at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Log of each scheduled or manual discovery run.
CREATE TABLE runs (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    trigger     TEXT NOT NULL, -- schedule | manual
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    fetched     INT NOT NULL DEFAULT 0,
    added       INT NOT NULL DEFAULT 0,
    errors      JSONB NOT NULL DEFAULT '[]'
);
