CREATE INDEX IF NOT EXISTS idx_emby_migration_item_matches_lux_item_id
    ON emby_migration_item_matches(lux_item_id);
CREATE INDEX IF NOT EXISTS idx_metadata_reidentify_jobs_library_id
    ON metadata_reidentify_jobs(library_id);
CREATE INDEX IF NOT EXISTS idx_playback_sessions_media_source_id
    ON playback_sessions(media_source_id);
CREATE INDEX IF NOT EXISTS idx_web_playback_sessions_media_source_id
    ON web_playback_sessions(media_source_id);
CREATE INDEX IF NOT EXISTS idx_scan_manifest_deltas_parent_key
    ON scan_manifest_deltas(manifest_id, library_root_id, relative_path, observation_sequence);
