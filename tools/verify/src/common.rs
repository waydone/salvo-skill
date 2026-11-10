//! Placeholders for application-side names the skill snippets reference but do not define
//! (handlers, DTOs, pools, a router), plus imports that docs show once per page.
//! Never Salvo API shims: anything Salvo-shaped must come from salvo itself.
use salvo::prelude::*;

pub use serde::{Deserialize, Serialize};
pub use serde_json::json;
pub use std::time::Duration;
pub use salvo::extract::{FormBody, HeaderParam, JsonBody, PathParam, QueryParam};

#[derive(Debug, Clone, Default, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct NewUser {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct MyForm {
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct MyData {
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyQuery {
    pub q: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MyType;

#[derive(Debug, Clone, Default)]
pub struct Config;

#[derive(Debug, Clone, Default)]
pub struct PgPool;

/// A do-nothing middleware value standing in for `jwt_auth`, `rate_limiter`, ...
#[derive(Clone, Copy)]
pub struct Noop;
#[async_trait]
impl Handler for Noop {
    async fn handle(&self, _: &mut Request, _: &mut Depot, _: &mut Response, _: &mut FlowCtrl) {}
}

pub fn ph_router() -> Router {
    Router::new()
}
pub fn router() -> Router {
    Router::new()
}
pub fn api_routes() -> Router {
    Router::new()
}
pub async fn ph_handle() -> salvo::server::ServerHandle {
    let acceptor = TcpListener::new("127.0.0.1:0").bind().await;
    Server::new(acceptor).handle()
}
pub fn ph_stream() -> impl futures_util::Stream<Item = Result<bytes::Bytes, std::convert::Infallible>> + Send + 'static {
    futures_util::stream::empty()
}

pub use salvo::test::{ResponseExt, TestClient};

pub mod db {
    pub async fn find(_id: i64) -> Result<Option<super::User>, anyhow::Error> {
        Ok(None)
    }
}
pub mod v1 {
    pub use super::{create, list};
    pub fn router() -> salvo::Router {
        salvo::Router::new()
    }
}
pub mod v2 {
    pub use super::{create, list};
    pub fn router() -> salvo::Router {
        salvo::Router::new()
    }
}
pub fn tenant_routes() -> Router {
    Router::new()
}
pub fn jwt_auth() -> Noop {
    Noop
}
pub fn mint_test_token() -> String {
    String::new()
}

macro_rules! ph_handlers {
    ($($n:ident),* $(,)?) => { $( #[handler] pub async fn $n() -> &'static str { "ok" } )* };
}
ph_handlers!(
    show, serve, by_id, index, list_users, create_user, get_user, update_user, delete_user,
    del_user, list_posts, hello, home, form_page, submit, handler, health, healthz, expensive_list,
    list, create, update, patch, remove, link_handler, search_handler, admin_panel, stats, me,
    login, connect, ws_handler, sse_handler, upload, download, serve_asset, index_page,
    admin_stats, public_index, private_index
);
