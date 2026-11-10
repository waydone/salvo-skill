//! Runtime checks for behavior claims the skill makes about Salvo 1.1.1.
#[cfg(test)]
mod tests {
    use salvo::extract::{FormFile, FormFiles};
    use salvo::http::FormLimits;
    use salvo::oapi::{OpenApi, OpenApiVersion};
    use salvo::prelude::*;
    use salvo::serve_static::StaticFile;
    use salvo::test::{ResponseExt, TestClient};

    #[handler]
    async fn only_get() -> &'static str {
        "ok"
    }

    /// routing.md / SKILL.md: wrong method on a matched path -> 405 + Allow (1.1).
    #[tokio::test]
    async fn method_mismatch_is_405_with_allow() {
        let service = Service::new(Router::with_path("items").get(only_get));
        let res = TestClient::post("http://localhost/items").send(&service).await;
        assert_eq!(res.status_code, Some(StatusCode::METHOD_NOT_ALLOWED));
        let allow = res.headers().get("allow").expect("Allow header").to_str().unwrap().to_owned();
        assert!(allow.contains("GET"), "Allow = {allow}");
    }

    fn multipart(files: &[(&str, &str)]) -> (String, Vec<u8>) {
        let boundary = "SALVOTESTBOUNDARY";
        let mut body = Vec::new();
        for (field, name) in files {
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{name}\"\r\nContent-Type: image/png\r\n\r\nfake-bytes\r\n"
                )
                .as_bytes(),
            );
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={boundary}"), body)
    }

    #[handler]
    async fn upload(avatar: FormFile) -> String {
        format!("{}:{}", avatar.name().unwrap_or("-"), avatar.size())
    }

    #[handler]
    async fn upload_many(attachments: FormFiles) -> String {
        attachments.len().to_string()
    }

    /// testing.md multipart body + data-extraction.md `FormFile` param named after the field.
    #[tokio::test]
    async fn hand_built_multipart_reaches_form_file() {
        let service = Service::new(Router::with_path("upload").post(upload));
        let (ct, body) = multipart(&[("avatar", "avatar.png")]);
        let mut res = TestClient::post("http://localhost/upload")
            .add_header("content-type", ct, true)
            .body(body)
            .send(&service)
            .await;
        assert_eq!(res.status_code, Some(StatusCode::OK));
        assert_eq!(res.take_string().await.unwrap(), "avatar.png:10");
    }

    /// data-extraction.md / files.md: exceeding FormLimits -> 413 (1.1).
    #[tokio::test]
    async fn form_limits_overflow_is_413() {
        let service = Service::new(
            Router::with_path("upload").hoop(FormLimits::new().max_files(1)).post(upload_many),
        );
        let (ct, body) = multipart(&[("attachments", "a.png"), ("attachments", "b.png")]);
        let res = TestClient::post("http://localhost/upload")
            .add_header("content-type", ct, true)
            .body(body)
            .send(&service)
            .await;
        assert_eq!(res.status_code, Some(StatusCode::PAYLOAD_TOO_LARGE));
    }

    /// files.md: SVG served as attachment + nosniff by default (1.0).
    #[tokio::test]
    async fn svg_is_attachment_by_default() {
        let dir = std::env::temp_dir().join("salvo-skill-verify-svg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("logo.svg");
        std::fs::write(&path, "<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap();
        let service = Service::new(Router::with_path("logo.svg").get(StaticFile::new(&path)));
        let res = TestClient::get("http://localhost/logo.svg").send(&service).await;
        assert_eq!(res.status_code, Some(StatusCode::OK));
        let cd = res.headers().get("content-disposition").map(|v| v.to_str().unwrap().to_owned());
        assert!(cd.as_deref().is_some_and(|v| v.starts_with("attachment")), "Content-Disposition = {cd:?}");
    }

    /// soybean/auth-security: app-level `jsonwebtoken = "11"` (default features = no crypto backend)
    /// unifies with Salvo's copy and inherits its backend, so encode/decode work without panicking.
    #[test]
    fn app_jsonwebtoken_11_inherits_salvo_backend() {
        use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
        #[derive(serde::Serialize, serde::Deserialize)]
        struct C {
            sub: String,
            exp: i64,
        }
        let exp = chrono::Utc::now().timestamp() + 3600;
        let tok = encode(&Header::new(Algorithm::HS256), &C { sub: "u".into(), exp }, &EncodingKey::from_secret(b"k")).unwrap();
        let back = decode::<C>(&tok, &DecodingKey::from_secret(b"k"), &Validation::new(Algorithm::HS256)).unwrap();
        assert_eq!(back.claims.sub, "u");
    }

    #[endpoint]
    async fn search() -> &'static str {
        "found"
    }

    /// openapi.md / routing.md: QUERY routes emitted only into a 3.2 document (1.0).
    #[tokio::test]
    async fn query_route_only_in_3_2_doc() {
        let router = Router::with_path("search").query(search);
        let v31 = serde_json::to_value(OpenApi::new("t", "1").merge_router(&router)).unwrap();
        let v32 = serde_json::to_value(
            OpenApi::new("t", "1").openapi_version(OpenApiVersion::Version3_2).merge_router(&router),
        )
        .unwrap();
        assert!(v31["paths"]["/search"]["query"].is_null(), "3.1 doc: {v31}");
        assert!(v32["paths"]["/search"]["query"].is_object(), "3.2 doc: {v32}");
    }
}
