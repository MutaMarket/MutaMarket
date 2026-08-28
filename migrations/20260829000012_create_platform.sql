CREATE TABLE app_settings (
    key text NOT NULL,
    value text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY app_settings
    ADD CONSTRAINT app_settings_pkey PRIMARY KEY (key);

CREATE TABLE scheduler_jobs (
    job text NOT NULL,
    paused boolean DEFAULT false NOT NULL
);

ALTER TABLE ONLY scheduler_jobs
    ADD CONSTRAINT scheduler_jobs_pkey PRIMARY KEY (job);

CREATE TABLE scheduler_runs (
    id bigserial NOT NULL,
    job text NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    finished_at timestamp with time zone,
    outcome text,
    summary text,
    error text,
    items bigint,
    metrics jsonb
);

ALTER TABLE ONLY scheduler_runs
    ADD CONSTRAINT scheduler_runs_pkey PRIMARY KEY (id);

CREATE INDEX scheduler_runs_job_index ON scheduler_runs USING btree (job, id DESC);

CREATE TABLE metric_samples (
    id bigserial NOT NULL,
    metric text NOT NULL,
    taken_at timestamp with time zone DEFAULT now() NOT NULL,
    value double precision NOT NULL
);

ALTER TABLE ONLY metric_samples
    ADD CONSTRAINT metric_samples_pkey PRIMARY KEY (id);

CREATE INDEX metric_samples_metric_taken_at_index ON metric_samples USING btree (metric, taken_at);

CREATE TABLE donations (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    journal_id bigint,
    amount double precision NOT NULL,
    date timestamp with time zone NOT NULL,
    confirmation_sent boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY donations
    ADD CONSTRAINT donations_pkey PRIMARY KEY (id);

CREATE INDEX donations_character_id_index ON donations USING btree (character_id);

CREATE INDEX donations_confirmation_sent_index ON donations USING btree (confirmation_sent);

CREATE INDEX donations_date_index ON donations USING btree (date);

CREATE INDEX donations_journal_id_index ON donations USING btree (journal_id);

CREATE TABLE eve_mails (
    id bigint NOT NULL,
    character_id bigint NOT NULL,
    is_read boolean DEFAULT false NOT NULL,
    subject text NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    body text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY eve_mails
    ADD CONSTRAINT eve_mails_pkey PRIMARY KEY (id);

CREATE INDEX eve_mails_is_read_idx ON eve_mails USING btree (is_read);

CREATE INDEX eve_mails_timestamp_idx ON eve_mails USING btree ("timestamp");

CREATE TABLE eve_mail_recipients (
    id bigserial NOT NULL,
    eve_mail_id bigint NOT NULL,
    character_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY eve_mail_recipients
    ADD CONSTRAINT eve_mail_recipients_eve_mail_id_character_id_key UNIQUE (eve_mail_id, character_id);

ALTER TABLE ONLY eve_mail_recipients
    ADD CONSTRAINT eve_mail_recipients_pkey PRIMARY KEY (id);

CREATE TABLE eve_mail_module (
    id bigserial NOT NULL,
    eve_mail_id bigint NOT NULL,
    module_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY eve_mail_module
    ADD CONSTRAINT eve_mail_module_eve_mail_id_module_id_key UNIQUE (eve_mail_id, module_id);

ALTER TABLE ONLY eve_mail_module
    ADD CONSTRAINT eve_mail_module_pkey PRIMARY KEY (id);

ALTER TABLE ONLY donations
    ADD CONSTRAINT donations_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE ONLY eve_mail_module
    ADD CONSTRAINT eve_mail_module_eve_mail_id_fkey FOREIGN KEY (eve_mail_id) REFERENCES eve_mails(id) ON DELETE CASCADE;

ALTER TABLE ONLY eve_mail_module
    ADD CONSTRAINT eve_mail_module_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;

ALTER TABLE ONLY eve_mail_recipients
    ADD CONSTRAINT eve_mail_recipients_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY eve_mail_recipients
    ADD CONSTRAINT eve_mail_recipients_eve_mail_id_fkey FOREIGN KEY (eve_mail_id) REFERENCES eve_mails(id) ON DELETE CASCADE;

ALTER TABLE ONLY eve_mails
    ADD CONSTRAINT eve_mails_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;
