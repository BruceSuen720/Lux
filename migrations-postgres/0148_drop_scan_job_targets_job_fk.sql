-- scan_job_targets are retained and cleaned through their owning scan job's
-- application lifecycle; avoid one parent-key probe per inserted target row.
ALTER TABLE scan_job_targets
    DROP CONSTRAINT IF EXISTS scan_job_targets_job_id_fkey;
