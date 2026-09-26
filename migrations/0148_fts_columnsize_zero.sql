-- Lux search uses MATCH only; it does not read FTS5 token counts or ranking.
-- Rebuild the index without the unused per-row column-size shadow table.
DROP TABLE media_search;

CREATE VIRTUAL TABLE media_search USING fts5(
    item_id UNINDEXED,
    title,
    sort_title,
    original_title,
    aliases,
    columnsize=0
);

INSERT INTO media_search (item_id, title, sort_title, original_title, aliases)
SELECT
    mi.id,
    mi.title,
    CASE
        WHEN mi.sort_title = mi.title COLLATE NOCASE THEN ''
        ELSE mi.sort_title
    END,
    COALESCE(mi.original_title, ''),
    COALESCE((SELECT group_concat(alias, ' ') FROM item_aliases WHERE item_id = mi.id), '')
FROM media_items mi;
