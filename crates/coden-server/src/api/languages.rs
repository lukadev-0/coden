use std::sync::Arc;

use axum::{extract::State, http::header, response::IntoResponse};
use serde::Deserialize;
use utoipa::{PartialSchema, ToSchema};

use crate::api::ApiState;

#[expect(unused)]
#[derive(Deserialize, ToSchema)]
pub struct Language {
    pub label: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub package: LanguagePackage,
    pub source_file: String,
    pub compile: Option<LanguageCommand>,
    pub execute: LanguageCommand,
}

#[expect(unused)]
#[derive(Deserialize, ToSchema)]
pub struct LanguagePackage {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct LanguageCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl TryFrom<Vec<String>> for LanguageCommand {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        let mut iter = value.into_iter();
        let program = iter
            .next()
            .clone()
            .ok_or("command must have at least one element")?;
        Ok(LanguageCommand {
            program,
            args: iter.collect(),
        })
    }
}

impl PartialSchema for LanguageCommand {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::schema!(Vec<String>).into()
    }
}
impl ToSchema for LanguageCommand {}

/// Get languages
#[utoipa::path(
    get,
    path = "/api/languages",
    responses(
        (status = OK, body = Vec<Language>, description = "The supported languages.")
    ),
)]
pub async fn get_languages(State(api_state): State<Arc<ApiState>>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        api_state.languages_manifest.clone(),
    )
}
