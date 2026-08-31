//! The OpenAPI description of the public API, derived from the handlers.
//!
//! Nothing here restates a path, a status or a field: the paths come from
//! the `#[utoipa::path]` annotations on the handlers, and the schemas from
//! the `ToSchema` derives on the response DTOs in
//! [`crate::view::public_api`]. Adding a field to a DTO changes the spec;
//! removing one removes it from the spec. There is no copy to keep in
//! sync, which is the whole point.
//!
//! Only the documented public endpoints appear. Everything else under
//! `/api` serves the site itself and is deliberately absent.

use utoipa::OpenApi;

use crate::view::public_api::{
    AbyssalTypeStatistic, ApiError, EstimatorStatistic, ImportModuleRequest, ModuleEnvelope,
    ModuleOrPage, ModulePage, PageLinks, PageMeta, StatisticAttribute, StatisticType,
    StatisticUnit, ValidationError,
};

/// The public base URL of the API, used as the spec's only server.
const PUBLIC_SERVER: &str = "https://mutamarket.com/api";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "MutaMarket API",
        version = "1.0.0",
        description = "Public API for abyssal modules in EVE Online: browse what is for sale, look up a single module with every rolled attribute and its estimated value, import a module from EVE, and read the reference data behind the roll-quality metrics.\n\nNo key and no account are needed. Please send a User-Agent that identifies you, ideally with a contact address, and do not call POST /api/modules in a loop: it calls EVE's ESI and runs a price model on every request.\n\nOnly the endpoints described here are public. Everything else under /api serves mutamarket.com itself and changes without notice.",
        contact(name = "MutaMarket", url = "https://mutamarket.com/documentation/support"),
        license(name = "See the site's legal page", url = "https://mutamarket.com/documentation/legal"),
    ),
    paths(
        super::api::modules_index_root,
        super::api::modules_show_or_index,
        super::api::store_module,
        super::api::estimator_statistics,
        super::api::abyssal_type_statistics,
    ),
    components(schemas(
        ModuleOrPage,
        ModuleEnvelope,
        ModulePage,
        PageLinks,
        PageMeta,
        ApiError,
        ValidationError,
        ImportModuleRequest,
        EstimatorStatistic,
        AbyssalTypeStatistic,
        StatisticAttribute,
        StatisticUnit,
        StatisticType,
    )),
    tags(
        (name = "Modules", description = "Browse, retrieve and import abyssal modules."),
        (name = "Reference", description = "Model quality and attribute roll ranges."),
    ),
    servers((url = PUBLIC_SERVER)),
)]
pub struct PublicApi;

/// The generated document, built once.
pub fn document() -> &'static utoipa::openapi::OpenApi {
    static DOCUMENT: std::sync::OnceLock<utoipa::openapi::OpenApi> = std::sync::OnceLock::new();
    DOCUMENT.get_or_init(PublicApi::openapi)
}
