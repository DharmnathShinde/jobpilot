-- Shared catalog of company career pages. Every user follows all of it by default;
-- job_sources stays for pages a user adds just for themselves.

CREATE TABLE companies (
    id              UUID PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('greenhouse', 'lever', 'ashby')),
    board           TEXT NOT NULL,
    name            TEXT NOT NULL,
    origin          TEXT NOT NULL DEFAULT 'seed', -- seed | request
    active          BOOLEAN NOT NULL DEFAULT true,
    job_count       INT NOT NULL DEFAULT 0,
    fail_count      INT NOT NULL DEFAULT 0,
    last_checked_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (kind, board)
);
CREATE INDEX companies_stale_idx ON companies (active, last_checked_at NULLS FIRST);

-- Open roles on catalog boards, refreshed by the cron. Users' runs filter and score from here.
CREATE TABLE catalog_jobs (
    company_id  UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    external_id TEXT NOT NULL,
    title       TEXT NOT NULL,
    location    TEXT NOT NULL DEFAULT '',
    url         TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    posted_at   TIMESTAMPTZ,
    seen_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (company_id, external_id)
);

-- "Add more career pages": users name a company or paste a link; the cron finds its board.
CREATE TABLE company_requests (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    query       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'added', 'not_found')),
    company_id  UUID REFERENCES companies(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ
);
CREATE INDEX company_requests_pending_idx ON company_requests (status, created_at);
