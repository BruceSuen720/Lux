CREATE TABLE scan_local_metadata_backfills (
    library_root_id TEXT PRIMARY KEY NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
    cursor_entry_id TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING'
        CHECK (status IN ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED')),
    attempts BIGINT NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    next_attempt_at BIGINT,
    error TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT)
);

CREATE INDEX idx_scan_local_metadata_backfills_claim
    ON scan_local_metadata_backfills(status, next_attempt_at, updated_at, library_root_id)
    WHERE status IN ('PENDING', 'FAILED');

CREATE INDEX idx_scan_local_metadata_backfills_recovery
    ON scan_local_metadata_backfills(status, updated_at, library_root_id)
    WHERE status = 'RUNNING';

CREATE INDEX idx_filesystem_entries_local_metadata_backfill
    ON filesystem_entries(library_root_id, id)
    WHERE entry_kind = 'FILE' AND is_missing = 0;
