-- Premium users can set a custom accent color that retints the whole
-- theme (everything derived from --primary). Stored per account; only
-- applied while the account has active premium.
ALTER TABLE users ADD COLUMN accent_color text;
