DROP INDEX IF EXISTS idx_metadata_reidentify_items_status;

ALTER TABLE metadata_reidentify_job_items
    ADD COLUMN priority INTEGER NOT NULL DEFAULT 3
        CHECK (priority BETWEEN 0 AND 3);

UPDATE metadata_reidentify_job_items
SET priority = CASE (
        SELECT item_type
        FROM media_items
        WHERE media_items.id = metadata_reidentify_job_items.item_id
    )
    WHEN 'MOVIE' THEN 0
    WHEN 'SERIES' THEN 0
    WHEN 'SEASON' THEN 1
    WHEN 'EPISODE' THEN 2
    ELSE 3
END;

CREATE INDEX idx_metadata_reidentify_items_claim_priority
    ON metadata_reidentify_job_items(job_id, status, priority, item_id);
