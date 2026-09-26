DROP TRIGGER IF EXISTS media_items_search_ai;
DROP TRIGGER IF EXISTS media_items_search_au;

-- FTS5 already folds ASCII case, so keep a second sort-title token stream only
-- when the display title and sort title differ by more than ASCII casing.
CREATE TRIGGER media_items_search_ai AFTER INSERT ON media_items BEGIN
    INSERT INTO media_search (item_id, title, sort_title, original_title, aliases)
    VALUES (
        NEW.id,
        NEW.title,
        CASE
            WHEN NEW.sort_title = NEW.title COLLATE NOCASE THEN ''
            ELSE NEW.sort_title
        END,
        COALESCE(NEW.original_title, ''),
        ''
    );
END;

CREATE TRIGGER media_items_search_au AFTER UPDATE OF title, sort_title, original_title ON media_items BEGIN
    DELETE FROM media_search WHERE item_id = OLD.id;
    INSERT INTO media_search (item_id, title, sort_title, original_title, aliases)
    VALUES (
        NEW.id,
        NEW.title,
        CASE
            WHEN NEW.sort_title = NEW.title COLLATE NOCASE THEN ''
            ELSE NEW.sort_title
        END,
        COALESCE(NEW.original_title, ''),
        COALESCE((SELECT group_concat(alias, ' ') FROM item_aliases WHERE item_id = NEW.id), '')
    );
END;
