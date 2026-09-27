ALTER TABLE libraries
    DROP CONSTRAINT libraries_kind_check,
    ADD CONSTRAINT libraries_kind_check
        CHECK (kind IN ('MOVIE', 'SERIES', 'MIXED', 'HOMEVIDEOS'));

ALTER TABLE media_items
    DROP CONSTRAINT media_items_item_type_check,
    ADD CONSTRAINT media_items_item_type_check
        CHECK (item_type IN (
            'MOVIE', 'SERIES', 'SEASON', 'EPISODE', 'BOX_SET', 'FOLDER', 'UNRESOLVED', 'VIDEO'
        ));
