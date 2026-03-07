-- Add CHECK constraints on text columns to match Rust validation rules.
-- Defense-in-depth: these mirror the `validator` crate limits on the API layer.

ALTER TABLE teams
  ADD CONSTRAINT chk_teams_tname_length CHECK (char_length(tname) <= 255);

ALTER TABLE teams
  ADD CONSTRAINT chk_teams_descr_length CHECK (char_length(descr) <= 1000);

ALTER TABLE roles
  ADD CONSTRAINT chk_roles_title_length CHECK (char_length(title) <= 255);

ALTER TABLE items
  ADD CONSTRAINT chk_items_descr_length CHECK (char_length(descr) <= 255);
