use std::time::Duration;

use axum::extract::Multipart;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        admin::{crud, model::NewAdminUploadAssetRecord, schema::AdminImageUploadResponse},
        auth::error::AuthError,
    },
    service::jwt::AuthenticatedUser,
};

const DEFAULT_FILEBASE_RPC_URL: &str = "https://rpc.filebase.io";
const DEFAULT_FILEBASE_GATEWAY_BASE_URL: &str = "https://ipfs.filebase.io/ipfs";
const DEFAULT_UPLOAD_SCOPE: &str = "events";
const MAX_IMAGE_SIZE_BYTES: usize = 10 * 1024 * 1024;

const ALLOWED_IMAGE_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/gif",
    "image/svg+xml",
    "image/avif",
];

pub async fn upload_admin_image(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    multipart: Multipart,
) -> Result<AdminImageUploadResponse, AuthError> {
    let payload = parse_image_upload(multipart).await?;
    persist_admin_image_upload(state, authenticated_user, payload).await
}

pub async fn upload_admin_image_file(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    raw_scope: Option<&str>,
    raw_file_name: &str,
    content_type: &str,
    bytes: Vec<u8>,
) -> Result<AdminImageUploadResponse, AuthError> {
    let payload = build_image_upload_payload(
        raw_scope,
        Some(raw_file_name),
        content_type.to_owned(),
        bytes,
    )?;

    persist_admin_image_upload(state, authenticated_user, payload).await
}

struct FilebaseUploadConfig {
    bucket_name: String,
    rpc_url: String,
    rpc_token: String,
    gateway_base_url: String,
}

impl FilebaseUploadConfig {
    fn from_state(state: &AppState) -> Result<Self, AuthError> {
        let bucket_name =
            state.env.filebase_bucket_name.clone().ok_or_else(|| {
                AuthError::internal("missing FILEBASE_BUCKET_NAME", "missing config")
            })?;

        let rpc_token = state.env.filebase_ipfs_rpc_token.clone().ok_or_else(|| {
            AuthError::internal("missing FILEBASE_IPFS_RPC_TOKEN", "missing config")
        })?;

        Ok(Self {
            bucket_name,
            rpc_url: state
                .env
                .filebase_ipfs_rpc_url
                .clone()
                .unwrap_or_else(|| DEFAULT_FILEBASE_RPC_URL.to_owned()),
            rpc_token,
            gateway_base_url: state
                .env
                .filebase_gateway_base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_FILEBASE_GATEWAY_BASE_URL.to_owned()),
        })
    }
}

struct ImageUploadPayload {
    scope: String,
    file_name: String,
    content_type: String,
    size_bytes: usize,
    bytes: Vec<u8>,
}

async fn parse_image_upload(mut multipart: Multipart) -> Result<ImageUploadPayload, AuthError> {
    let mut scope = DEFAULT_UPLOAD_SCOPE.to_owned();
    let mut raw_file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AuthError::bad_request(format!("invalid multipart body: {error}")))?
    {
        let field_name = field.name().unwrap_or_default().to_owned();

        match field_name.as_str() {
            "scope" | "folder" => {
                let raw_scope = field.text().await.map_err(|error| {
                    AuthError::bad_request(format!("invalid scope field: {error}"))
                })?;

                if !raw_scope.trim().is_empty() {
                    scope = sanitize_scope(&raw_scope);
                }
            }
            "file" => {
                if bytes.is_some() {
                    return Err(AuthError::bad_request(
                        "only one file can be uploaded per request",
                    ));
                }

                let detected_content_type = field
                    .content_type()
                    .map(str::to_owned)
                    .ok_or_else(|| AuthError::bad_request("file content type is required"))?;

                validate_content_type(&detected_content_type)?;

                let field_file_name = field.file_name().map(str::to_owned);
                let field_bytes = field.bytes().await.map_err(|error| {
                    AuthError::bad_request(format!("invalid file payload: {error}"))
                })?;

                raw_file_name = field_file_name;
                content_type = Some(detected_content_type);
                bytes = Some(field_bytes.to_vec());
            }
            _ => {}
        }
    }

    let bytes =
        bytes.ok_or_else(|| AuthError::bad_request("multipart field `file` is required"))?;
    let content_type =
        content_type.ok_or_else(|| AuthError::bad_request("file content type is required"))?;

    build_image_upload_payload(Some(&scope), raw_file_name.as_deref(), content_type, bytes)
}

fn validate_content_type(content_type: &str) -> Result<(), AuthError> {
    if ALLOWED_IMAGE_CONTENT_TYPES.contains(&content_type) {
        return Ok(());
    }

    Err(AuthError::bad_request(format!(
        "unsupported image content type `{content_type}`"
    )))
}

fn build_image_upload_payload(
    raw_scope: Option<&str>,
    raw_file_name: Option<&str>,
    content_type: String,
    bytes: Vec<u8>,
) -> Result<ImageUploadPayload, AuthError> {
    validate_content_type(&content_type)?;

    if bytes.is_empty() {
        return Err(AuthError::bad_request("uploaded file cannot be empty"));
    }

    if bytes.len() > MAX_IMAGE_SIZE_BYTES {
        return Err(AuthError::bad_request(format!(
            "image exceeds max size of {MAX_IMAGE_SIZE_BYTES} bytes"
        )));
    }

    let scope = raw_scope
        .filter(|value| !value.trim().is_empty())
        .map(sanitize_scope)
        .unwrap_or_else(|| DEFAULT_UPLOAD_SCOPE.to_owned());
    let file_name = build_upload_file_name(raw_file_name, &content_type);

    Ok(ImageUploadPayload {
        scope,
        file_name,
        content_type,
        size_bytes: bytes.len(),
        bytes,
    })
}

fn sanitize_scope(raw_scope: &str) -> String {
    let sanitized = raw_scope
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    let collapsed = sanitized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if collapsed.is_empty() {
        DEFAULT_UPLOAD_SCOPE.to_owned()
    } else {
        collapsed
    }
}

fn build_upload_file_name(original_name: Option<&str>, content_type: &str) -> String {
    let extension = infer_extension(original_name, content_type);
    let sanitized = original_name
        .map(sanitize_file_name)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("upload.{extension}"));

    format!("{}-{sanitized}", Uuid::new_v4())
}

fn sanitize_file_name(raw_name: &str) -> String {
    raw_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw_name)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn infer_extension(original_name: Option<&str>, content_type: &str) -> &'static str {
    if let Some(extension) = original_name
        .and_then(|name| name.rsplit('.').next())
        .map(|value| value.to_ascii_lowercase())
    {
        match extension.as_str() {
            "jpg" | "jpeg" => return "jpg",
            "png" => return "png",
            "webp" => return "webp",
            "gif" => return "gif",
            "svg" => return "svg",
            "avif" => return "avif",
            _ => {}
        }
    }

    match content_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/avif" => "avif",
        _ => "bin",
    }
}

async fn upload_to_filebase(
    state: &AppState,
    config: &FilebaseUploadConfig,
    payload: &ImageUploadPayload,
) -> Result<FilebaseAddResponse, AuthError> {
    let upload_url = format!(
        "{}/api/v0/add?cid-version=1",
        config.rpc_url.trim_end_matches('/')
    );
    let file_part = Part::bytes(payload.bytes.clone())
        .file_name(payload.file_name.clone())
        .mime_str(&payload.content_type)
        .map_err(|error| AuthError::bad_request(format!("invalid image content type: {error}")))?;
    let form = Form::new().part("file", file_part);

    let response = state
        .http_client
        .post(upload_url)
        .bearer_auth(&config.rpc_token)
        .multipart(form)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| AuthError::internal("filebase upload request failed", error))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AuthError::internal("filebase upload response read failed", error))?;

    if !status.is_success() {
        tracing::error!(status = %status, body, "filebase upload failed");
        return Err(AuthError::internal(
            "filebase upload returned an error",
            status,
        ));
    }

    let payload = body
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| AuthError::internal("filebase upload returned empty body", "empty body"))?;

    serde_json::from_str::<FilebaseAddResponse>(payload)
        .map_err(|error| AuthError::internal("invalid filebase upload response", error))
}

async fn persist_admin_image_upload(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    payload: ImageUploadPayload,
) -> Result<AdminImageUploadResponse, AuthError> {
    let config = FilebaseUploadConfig::from_state(state)?;
    let response = upload_to_filebase(state, &config, &payload).await?;
    let gateway_base_url = config.gateway_base_url.trim_end_matches('/');
    let record = crud::create_admin_upload_asset(
        &state.db,
        NewAdminUploadAssetRecord {
            id: Uuid::new_v4(),
            storage_provider: "filebase_ipfs".to_owned(),
            bucket_name: config.bucket_name,
            scope: payload.scope,
            file_name: payload.file_name,
            content_type: payload.content_type,
            size_bytes: payload.size_bytes as i64,
            cid: response.hash.clone(),
            ipfs_url: format!("ipfs://{}", response.hash),
            gateway_url: format!("{gateway_base_url}/{}", response.hash),
            created_by_user_id: authenticated_user.user_id,
        },
    )
    .await?;

    Ok(AdminImageUploadResponse::from_record(record))
}

#[derive(Debug, Deserialize)]
struct FilebaseAddResponse {
    #[serde(rename = "Hash")]
    hash: String,
}

#[cfg(test)]
mod tests {
    use super::{build_upload_file_name, sanitize_scope};

    #[test]
    fn sanitizes_scope_for_uploads() {
        assert_eq!(sanitize_scope("Event Images"), "event-images");
        assert_eq!(sanitize_scope(""), "events");
    }

    #[test]
    fn prefixes_generated_upload_file_name() {
        let file_name = build_upload_file_name(Some("my image.png"), "image/png");

        assert!(file_name.ends_with("my-image.png"));
        assert!(file_name.len() > "my-image.png".len());
    }
}
