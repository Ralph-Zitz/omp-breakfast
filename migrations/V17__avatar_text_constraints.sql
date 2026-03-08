-- V17: Add CHECK constraints on avatars text columns for consistency.
--
-- #691: name and content_type columns had no length constraints.
--       Avatars are only seeded from minifigs/ at startup, not user-provided,
--       but adding CHECK constraints maintains consistency with other tables.

ALTER TABLE avatars
    ADD CONSTRAINT avatars_name_length CHECK (char_length(name) <= 255);

ALTER TABLE avatars
    ADD CONSTRAINT avatars_content_type_length CHECK (char_length(content_type) <= 100);
