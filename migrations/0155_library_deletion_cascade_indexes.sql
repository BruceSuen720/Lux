CREATE INDEX IF NOT EXISTS idx_danmaku_match_job_items_media_source_id
    ON danmaku_match_job_items(media_source_id);
CREATE INDEX IF NOT EXISTS idx_chapter_detection_job_items_source_id
    ON chapter_detection_job_items(source_id);
CREATE INDEX IF NOT EXISTS idx_chapter_detection_job_items_item_id
    ON chapter_detection_job_items(item_id);
CREATE INDEX IF NOT EXISTS idx_chapter_detection_job_items_season_id
    ON chapter_detection_job_items(season_id);
CREATE INDEX IF NOT EXISTS idx_reconciliation_scan_entries_library_root_id
    ON reconciliation_scan_entries(library_root_id);
CREATE INDEX IF NOT EXISTS idx_scan_job_paths_library_root_id
    ON scan_job_paths(library_root_id);
CREATE INDEX IF NOT EXISTS idx_emby_migration_import_records_lux_item_id
    ON emby_migration_import_records(lux_item_id);
CREATE INDEX IF NOT EXISTS idx_user_item_state_item_id
    ON user_item_state(item_id);
CREATE INDEX IF NOT EXISTS idx_playback_sessions_item_id
    ON playback_sessions(item_id);
CREATE INDEX IF NOT EXISTS idx_chapter_detection_jobs_library_id
    ON chapter_detection_jobs(library_id);
CREATE INDEX IF NOT EXISTS idx_strm_probe_jobs_library_id
    ON strm_probe_jobs(library_id);
CREATE INDEX IF NOT EXISTS idx_web_playback_sessions_item_id
    ON web_playback_sessions(item_id);
CREATE INDEX IF NOT EXISTS idx_danmaku_match_jobs_library_id
    ON danmaku_match_jobs(library_id);
CREATE INDEX IF NOT EXISTS idx_scan_manifest_roots_library_root_id
    ON scan_manifest_roots(library_root_id);
CREATE INDEX IF NOT EXISTS idx_library_cover_jobs_library_id
    ON library_cover_jobs(library_id);
CREATE INDEX IF NOT EXISTS idx_user_library_order_library_id
    ON user_library_order(library_id);
CREATE INDEX IF NOT EXISTS idx_scan_local_metadata_batches_library_root_id
    ON scan_local_metadata_batches(library_root_id);
CREATE INDEX IF NOT EXISTS idx_scan_manifest_deltas_manifest_root_id
    ON scan_manifest_deltas(manifest_id, library_root_id);
