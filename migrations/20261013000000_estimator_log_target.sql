-- Whether a stored forest was fitted on ln(1 + price) instead of raw ISK.
--
-- New models train on the log scale (`training::TRAINING_TARGET`); the
-- forests already stored stay raw-ISK models and keep predicting as such
-- until the weekly training replaces them.
ALTER TABLE estimator_models ADD COLUMN log_target boolean NOT NULL DEFAULT false;
