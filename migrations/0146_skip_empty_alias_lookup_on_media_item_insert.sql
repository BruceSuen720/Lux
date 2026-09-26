DROP TRIGGER IF EXISTS media_items_search_ai;

CREATE TRIGGER media_items_search_ai AFTER INSERT ON media_items BEGIN
    INSERT INTO media_search (item_id, title, sort_title, original_title, aliases)
    VALUES (NEW.id, NEW.title, NEW.sort_title, COALESCE(NEW.original_title, ''), '');
END;
