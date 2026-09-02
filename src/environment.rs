//! The deployment environment switch, the legacy `config('app.env')`.
//! `APP_ENV=local` marks a developer machine: OpenGraph cards re-render
//! on every request and cookies stay usable over plain http. Anything
//! else, including an unset variable, is treated as production, like
//! Laravel's default.

pub const APP_ENV: &str = "APP_ENV";

const LOCAL: &str = "local";

pub fn is_local() -> bool {
    std::env::var(APP_ENV).is_ok_and(|env| env == LOCAL)
}
