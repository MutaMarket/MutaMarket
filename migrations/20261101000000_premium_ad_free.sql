-- The account's premium plan: ad-free premium hides the site's ads and
-- costs three times as much, so donations buy a third of the time and a
-- plan switch converts the remaining time of every character of the
-- account at that rate. One setting per account, so linked characters
-- never disagree; the premium time itself stays per character.
ALTER TABLE users ADD COLUMN premium_ad_free boolean DEFAULT false NOT NULL;
