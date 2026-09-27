-- media_search is maintained by media_items statement triggers, including delete cleanup.
-- Avoid one parent-row probe for every derived search row inserted during a scan.
ALTER TABLE media_search
    DROP CONSTRAINT IF EXISTS media_search_item_id_fkey;
