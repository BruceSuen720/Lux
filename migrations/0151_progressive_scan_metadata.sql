ALTER TABLE libraries
    ADD COLUMN scan_missing_metadata_auto_match_enabled INTEGER NOT NULL DEFAULT 1
        CHECK (scan_missing_metadata_auto_match_enabled IN (0, 1));

UPDATE libraries
SET scan_missing_metadata_auto_match_enabled = realtime_metadata_auto_match_enabled;

CREATE TABLE scan_local_metadata_batches (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL,
    library_root_id TEXT NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
    batch_sequence INTEGER NOT NULL CHECK (batch_sequence >= 0),
    source_refs_json TEXT NOT NULL CHECK (length(source_refs_json) > 0),
    source_count INTEGER NOT NULL CHECK (source_count BETWEEN 1 AND 256),
    status TEXT NOT NULL DEFAULT 'PENDING'
        CHECK (status IN ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED', 'CANCELLED')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    next_attempt_at INTEGER,
    error TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE (job_id, library_root_id, batch_sequence)
);

CREATE INDEX idx_scan_local_metadata_batches_claim
    ON scan_local_metadata_batches(status, next_attempt_at, created_at, id)
    WHERE status IN ('PENDING', 'FAILED');

CREATE INDEX idx_scan_local_metadata_batches_recovery
    ON scan_local_metadata_batches(status, updated_at, id)
    WHERE status = 'RUNNING';

CREATE TABLE item_metadata_completeness (
    item_id TEXT NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    capability TEXT NOT NULL CHECK (length(trim(capability)) BETWEEN 1 AND 64),
    local_state TEXT NOT NULL
        CHECK (local_state IN ('PENDING', 'RUNNING', 'READY', 'FAILED', 'CANCELLED')),
    is_missing INTEGER CHECK (is_missing IN (0, 1)),
    input_fingerprint BLOB,
    checked_at INTEGER,
    retry_after INTEGER,
    error TEXT,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (item_id, capability),
    CHECK (
        (local_state = 'READY' AND is_missing IS NOT NULL)
        OR (local_state <> 'READY' AND is_missing IS NULL)
    )
);

CREATE INDEX idx_item_metadata_completeness_missing
    ON item_metadata_completeness(capability, item_id)
    WHERE local_state = 'READY' AND is_missing = 1;
