-- New scanned items usually have no provider IDs. Keep them out of the JSON
-- table function instead of expanding an empty object for every inserted row.
CREATE OR REPLACE FUNCTION lux_refresh_media_search_items_insert_stmt()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO media_search (item_id, title, sort_title, original_title, aliases)
    SELECT n.id,
           n.title,
           n.sort_title,
           COALESCE(n.original_title, ''),
           ''
    FROM new_rows n;

    WITH provider_rows AS MATERIALIZED (
        SELECT n.id, n.item_type, n.provider_ids_json
        FROM new_rows n
        WHERE n.provider_ids_json IS NOT NULL
          AND n.provider_ids_json <> '{}'
    )
    INSERT INTO media_item_provider_ids (media_item_id, item_type, provider, provider_id)
    SELECT n.id, n.item_type, lower(providers.key), providers.value
    FROM provider_rows n
    CROSS JOIN LATERAL json_each_text(n.provider_ids_json::json) AS providers
    WHERE providers.value IS NOT NULL
    ON CONFLICT (media_item_id, provider, provider_id) DO NOTHING;
    RETURN NULL;
END;
$$;
