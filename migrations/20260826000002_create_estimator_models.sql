-- Trained per-type random forest models, replacing the legacy joblib
-- artifacts in estimators/models/. The bytes are the bincode-serialized
-- Forest (src/estimator/forest.rs); feature_names mirrors the artifact's
-- feature_names list (training column order).
create table estimator_models (
    type_id bigint primary key references types (id),
    feature_names jsonb not null,
    model bytea not null,
    trained_at timestamptz not null default now()
);
