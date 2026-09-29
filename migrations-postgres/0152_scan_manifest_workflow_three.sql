ALTER TABLE scan_manifests
    DROP CONSTRAINT IF EXISTS scan_manifests_workflow_version_check;

ALTER TABLE scan_manifests
    ADD CONSTRAINT scan_manifests_workflow_version_check
        CHECK (workflow_version IN (1, 2, 3));
