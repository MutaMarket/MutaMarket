//! The Leptos pages of the site. Pages carrying real data load it through
//! server functions; pages still being rebuilt render as placeholders until
//! their feature milestone lands.

mod documentation;
mod filters;
mod home;
mod layout;
mod login;
mod modules_page;
mod placeholder;

pub use documentation::DocumentationPage;
pub use home::HomePage;
pub use layout::Layout;
pub use login::LoginPage;
pub use modules_page::{AllModulesPage, ModulesPage};
pub use placeholder::PlaceholderPage;
