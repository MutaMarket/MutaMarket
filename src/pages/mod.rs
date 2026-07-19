//! The Leptos pages of the site. Pages carrying real data load it through
//! server functions; pages still being rebuilt render as placeholders until
//! their feature milestone lands.

mod character_menu;
mod documentation;
mod filter_controls;
mod filters;
mod home;
mod layout;
mod login;
mod modules_page;
pub mod personal_modules;
mod placeholder;
mod social_pages;
mod type_dialog;

pub use documentation::DocumentationPage;
pub use home::HomePage;
pub use layout::Layout;
pub use login::LoginPage;
pub use modules_page::{AllModulesPage, ModulesPage};
pub use personal_modules::PersonalModulesPage;
pub use placeholder::PlaceholderPage;
pub use social_pages::{CharacterPage, CharactersPage, CollectionPage, CollectionsPage};
