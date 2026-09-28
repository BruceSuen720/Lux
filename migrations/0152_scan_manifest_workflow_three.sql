-- no-transaction
PRAGMA foreign_keys = OFF;

CREATE TABLE scan_manifests_new (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL UNIQUE REFERENCES scan_jobs(id) ON DELETE CASCADE,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN (
        'DISCOVERING', 'READY_TO_DIFF', 'APPLYING', 'INDEXED',
        'POSTPROCESSING', 'COMPLETED', 'FAILED', 'CANCELLED'
    )),
    root_count INTEGER NOT NULL DEFAULT 0 CHECK (root_count >= 0),
    discovered_directory_count INTEGER NOT NULL DEFAULT 0 CHECK (discovered_directory_count >= 0),
    completed_directory_count INTEGER NOT NULL DEFAULT 0 CHECK (completed_directory_count >= 0),
    observed_file_count INTEGER NOT NULL DEFAULT 0 CHECK (observed_file_count >= 0),
    unchanged_count INTEGER NOT NULL DEFAULT 0 CHECK (unchanged_count >= 0),
    add_count INTEGER NOT NULL DEFAULT 0 CHECK (add_count >= 0),
    change_count INTEGER NOT NULL DEFAULT 0 CHECK (change_count >= 0),
    remove_count INTEGER NOT NULL DEFAULT 0 CHECK (remove_count >= 0),
    reappeared_count INTEGER NOT NULL DEFAULT 0 CHECK (reappeared_count >= 0),
    applied_delta_count INTEGER NOT NULL DEFAULT 0 CHECK (applied_delta_count >= 0),
    error TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    indexed_at INTEGER,
    completed_at INTEGER,
    resume_state TEXT,
    workflow_version INTEGER NOT NULL DEFAULT 1 CHECK (workflow_version IN (1, 2, 3)),
    discovery_format_version INTEGER NOT NULL DEFAULT 2 CHECK (discovery_format_version IN (2, 3)),
    discovery_mode TEXT NOT NULL DEFAULT 'PERSISTED' CHECK (discovery_mode IN ('PERSISTED', 'LITE')),
    postprocessing_targets_ready INTEGER NOT NULL DEFAULT 1 CHECK (postprocessing_targets_ready IN (0, 1))
);

INSERT INTO scan_manifests_new (
    id, job_id, library_id, state, root_count, discovered_directory_count,
    completed_directory_count, observed_file_count, unchanged_count, add_count,
    change_count, remove_count, reappeared_count, applied_delta_count, error,
    created_at, updated_at, indexed_at, completed_at, resume_state, workflow_version,
    discovery_format_version, discovery_mode, postprocessing_targets_ready
)
SELECT
    id, job_id, library_id, state, root_count, discovered_directory_count,
    completed_directory_count, observed_file_count, unchanged_count, add_count,
    change_count, remove_count, reappeared_count, applied_delta_count, error,
    created_at, updated_at, indexed_at, completed_at, resume_state, workflow_version,
    discovery_format_version, discovery_mode, postprocessing_targets_ready
FROM scan_manifests;

DROP TABLE scan_manifests;
ALTER TABLE scan_manifests_new RENAME TO scan_manifests;

CREATE INDEX idx_scan_manifests_library_state
    ON scan_manifests(library_id, state, updated_at, id);

PRAGMA foreign_keys = ON;
