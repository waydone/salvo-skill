//! Runtime checks for behavior claims the skill makes about Salvo 1.1.x.
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

    // ---- 1.1.2 ----

    #[handler]
    async fn tail(req: &mut Request) -> String {
        req.param::<String>("rest").unwrap_or_default()
    }

    /// routing.md: encoded path separators are rejected with 400 before routing (1.1.2).
    #[tokio::test]
    async fn encoded_separator_is_400() {
        static HOOPED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        #[handler]
        async fn count_hoop() {
            HOOPED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        let service = Service::new(Router::new().hoop(count_hoop).push(Router::with_path("files/{**rest}").get(tail)));
        let ok = TestClient::get("http://localhost/files/a/b").send(&service).await;
        assert_eq!(ok.status_code, Some(StatusCode::OK));
        assert_eq!(HOOPED.load(std::sync::atomic::Ordering::SeqCst), 1);
        for path in ["/files/a%2Fb", "/files/a%2fb", "/files/a%5Cb"] {
            let res = TestClient::get(format!("http://localhost{path}")).send(&service).await;
            assert_eq!(res.status_code, Some(StatusCode::BAD_REQUEST), "{path}");
        }
        // routing.md: the 400 is produced before any hoop runs.
        assert_eq!(HOOPED.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    /// auth-security.md: `CookieStore::host_prefixed()` issues a `__Host-` Secure cookie even over plain HTTP hops (1.1.2).
    #[tokio::test]
    async fn csrf_host_prefixed_cookie() {
        use salvo::csrf::{BcryptCipher, CookieStore, Csrf, FormFinder};
        let csrf = Csrf::new(BcryptCipher::new(), CookieStore::host_prefixed(), FormFinder::new("csrf_token"));
        let service = Service::new(Router::new().hoop(csrf).get(only_get));
        let res = TestClient::get("http://localhost/").send(&service).await;
        let cookie = res.headers().get("set-cookie").expect("Set-Cookie").to_str().unwrap().to_owned();
        assert!(cookie.starts_with("__Host-salvo.csrf="), "{cookie}");
        assert!(cookie.contains("Secure") && cookie.contains("Path=/") && !cookie.contains("Domain"), "{cookie}");
    }

    /// ops.md: `max_concurrency(n).include_body(true)` exists (1.1.2) and still serves.
    #[tokio::test]
    async fn concurrency_include_body() {
        use salvo::concurrency_limiter::max_concurrency;
        let service = Service::new(Router::new().hoop(max_concurrency(1).include_body(true)).get(only_get));
        for _ in 0..2 {
            let mut res = TestClient::get("http://localhost/").send(&service).await;
            assert_eq!(res.take_string().await.unwrap(), "ok");
        }
    }

    /// openapi.md: `Operation::request_body` takes a `Ref` (1.1.2); `Link` serializes camelCase names.
    #[test]
    fn oapi_request_body_ref_and_link_names() {
        use salvo::oapi::{Link, Operation, Ref, RefOr, RequestBody};
        let op = Operation::new().request_body(Ref::new("#/components/requestBodies/NewUser"));
        assert!(matches!(op.request_body, Some(RefOr::Ref(_))));
        let v = serde_json::to_value(&op).unwrap();
        assert_eq!(v["requestBody"]["$ref"], "#/components/requestBodies/NewUser");
        let inline = Operation::new().request_body(RequestBody::new());
        assert!(matches!(inline.request_body, Some(RefOr::Type(_))));
        let link = serde_json::to_value(Link::default().operation_id("read")).unwrap();
        assert_eq!(link["operationId"], "read");
    }

    static HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[handler]
    async fn counted() -> String {
        HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst).to_string()
    }

    /// ops.md: a request `Cache-Control: no-cache` bypasses the response cache (1.1.2).
    #[tokio::test]
    async fn cache_honors_request_no_cache() {
        use salvo::cache::{Cache, MokaStore, RequestIssuer};
        let cache = Cache::new(
            MokaStore::builder().time_to_live(std::time::Duration::from_secs(60)).build(),
            RequestIssuer::default(),
        );
        let service = Service::new(Router::new().hoop(cache).get(counted));
        let mut a = TestClient::get("http://localhost/").send(&service).await;
        let mut b = TestClient::get("http://localhost/").send(&service).await;
        assert_eq!(a.take_string().await.unwrap(), b.take_string().await.unwrap(), "second hit is cached");
        let before = HITS.load(std::sync::atomic::Ordering::SeqCst);
        TestClient::get("http://localhost/").add_header("cache-control", "no-cache", true).send(&service).await;
        assert_eq!(HITS.load(std::sync::atomic::Ordering::SeqCst), before + 1, "no-cache reaches the handler");
    }

    #[handler]
    async fn big() -> String {
        "salvo ".repeat(2000)
    }

    /// ops.md: an explicit higher-q `identity` wins the compression negotiation (1.1.2).
    #[tokio::test]
    async fn compression_respects_identity_preference() {
        use salvo::compression::{Compression, CompressionLevel};
        let comp = Compression::new().enable_gzip(CompressionLevel::Default);
        let service = Service::new(Router::new().hoop(comp).get(big));
        let gz = TestClient::get("http://localhost/").add_header("accept-encoding", "gzip", true).send(&service).await;
        assert_eq!(gz.headers().get("content-encoding").map(|v| v.to_str().unwrap()), Some("gzip"));
        let id = TestClient::get("http://localhost/")
            .add_header("accept-encoding", "gzip;q=0.5, identity;q=1", true)
            .send(&service)
            .await;
        assert!(id.headers().get("content-encoding").is_none(), "{:?}", id.headers());
    }
}
