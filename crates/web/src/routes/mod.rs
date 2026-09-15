// tecton-web/src/routes/mod.rs (updated)
pub mod health;
pub mod index;
pub mod search;
pub mod update;

//use crate::openapi::openapi_router;
use crate::state::AppState;

use axum::extract::State;
use axum::middleware::{self};
use axum::routing::get_service;
use axum::{Router, routing::get};

use tower_http::services::{ServeDir, ServeFile};

use std::future::ready;

use tecton_core::metrics::{TB_INDEX_NAME, setup_metrics_recorder, update_documents_total};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_scalar::{Scalar, Servable as ScalarServable};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(

    tags(
        (name = "index", description = "Document indexing endpoints"),
        (name = "search", description = "Search endpoints"),
    ),
    info(
        title = format!("{} API",TB_INDEX_NAME),
        version = "0.1.0",
        description = "tecton a text block indexing and search API with fulltext, vector, tree, and keyword search capabilities",
        contact(
            name = "tecton Team",
           // email = "support@example.com"
        ),
        license(
            name = "MIT OR Apache-2.0",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.example.com", description = "Production server")
    )
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_key",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("API_KEY")
                    .build(),
            ),
        );
    }
}

#[derive(Clone)]
struct OuterState {}

pub fn create_router(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/index", index::router())
        .nest("/api/search", search::router())
        .with_state(state)
        .split_for_parts();

    async fn outer_handler(_state: State<OuterState>) {}
    router
        .route_layer(middleware::from_fn(health::track_metrics))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
        .merge(Redoc::with_url("/redoc", api.clone()))
        //.merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        // Alternative to above
        .merge(RapiDoc::with_openapi("/api-docs/openapi2.json", api.clone()).path("/rapidoc"))
        .merge(Scalar::with_url("/scalar", api))
        .with_state(outer_handler)
}

pub fn create_service_router(state: AppState) -> Router {
    let recorder_handle = setup_metrics_recorder();
    //need to setup the documents count after metrics setup
    update_documents_total(state.index.num_docs());

    let h_routes = health::router(state);
    Router::new()
        .nest("/actuate", h_routes)
        .route("/metrics", get(move || ready(recorder_handle.render())))
        .route(
            "/favicon.ico",
            get_service(ServeFile::new("assets/favicon.ico")),
        )
        .nest_service("/assets", ServeDir::new("assets"))
}
