use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use task_manager::{AppState, app, config::Config, db};
use tower::ServiceExt;

const ADMIN_EMAIL: &str = "admin@example.com";
const ADMIN_PW: &str = "correct-horse-battery";

async fn setup_with(access_ttl: u64) -> Router {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    db::migrate(&pool).await.unwrap();
    db::create_first_admin(&pool, ADMIN_EMAIL, ADMIN_PW)
        .await
        .unwrap();
    let config = Config {
        database_url: "sqlite::memory:".into(),
        jwt_secret: "test-secret-test-secret-test-secret-123".into(),
        bind_addr: "127.0.0.1:0".into(),
        access_ttl: Duration::from_secs(access_ttl),
        refresh_ttl: Duration::from_secs(3600),
        static_dir: "static".into(),
    };
    app(AppState {
        pool,
        config: Arc::new(config),
    })
}

async fn setup() -> Router {
    setup_with(900).await
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let body = match body {
        Some(b) => {
            req = req.header("content-type", "application/json");
            Body::from(b.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn login(app: &Router, email: &str, pw: &str) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": email, "password": pw})),
    )
    .await
}

async fn admin_token(app: &Router) -> String {
    login(app, ADMIN_EMAIL, ADMIN_PW).await.1["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn login_success_and_failure() {
    let app = setup().await;
    let (s, body) = login(&app, "ADMIN@example.com", ADMIN_PW).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["user"]["roles"], json!(["admin"]));
    assert_eq!(
        login(&app, ADMIN_EMAIL, "wrong-password!").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(&app, "nobody@example.com", ADMIN_PW).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn me_requires_valid_token() {
    let app = setup().await;
    assert_eq!(
        call(&app, "GET", "/api/me", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, "GET", "/api/me", Some("garbage"), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let t = admin_token(&app).await;
    let (s, body) = call(&app, "GET", "/api/me", Some(&t), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["email"], ADMIN_EMAIL);
}

#[tokio::test]
async fn expired_token_rejected() {
    let app = setup_with(0).await;
    let t = admin_token(&app).await;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        call(&app, "GET", "/api/me", Some(&t), None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn refresh_rotation_and_reuse_detection() {
    let app = setup().await;
    let (_, first) = login(&app, ADMIN_EMAIL, ADMIN_PW).await;
    let r1 = first["refresh_token"].as_str().unwrap();
    let (s, second) = call(
        &app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": r1})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let r2 = second["refresh_token"].as_str().unwrap();
    assert_ne!(r1, r2);
    // Reusing the rotated token fails and burns the family, including r2.
    let (s, _) = call(
        &app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": r1})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(
        &app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": r2})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    // A JWT is not a refresh token.
    let (s, _) = call(
        &app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": first["access_token"]})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let app = setup().await;
    let (_, t) = login(&app, ADMIN_EMAIL, ADMIN_PW).await;
    let r = t["refresh_token"].clone();
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/auth/logout",
            None,
            Some(json!({"refresh_token": r}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/auth/refresh",
            None,
            Some(json!({"refresh_token": r}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn admin_manages_users_and_non_admin_is_blocked() {
    let app = setup().await;
    let admin = admin_token(&app).await;
    let (s, _) = call(
        &app,
        "POST",
        "/api/roles",
        Some(&admin),
        Some(json!({"name": "editor"})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, user) = call(&app, "POST", "/api/users", Some(&admin), Some(json!({
        "email": "bob@example.com", "password": "bobs-long-password", "display_name": "Bob", "roles": ["editor"]
    }))).await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(user["roles"], json!(["editor"]));
    let id = user["id"].as_i64().unwrap();

    let dup = call(
        &app,
        "POST",
        "/api/users",
        Some(&admin),
        Some(json!({
            "email": "BOB@example.com", "password": "bobs-long-password", "roles": ["editor"]
        })),
    )
    .await;
    assert_eq!(dup.0, StatusCode::CONFLICT);
    let bad_role = call(
        &app,
        "POST",
        "/api/users",
        Some(&admin),
        Some(json!({
            "email": "x@example.com", "password": "bobs-long-password", "roles": ["nope"]
        })),
    )
    .await;
    assert_eq!(bad_role.0, StatusCode::BAD_REQUEST);

    let (_, bob) = login(&app, "bob@example.com", "bobs-long-password").await;
    let bob_token = bob["access_token"].as_str().unwrap();
    assert_eq!(
        call(&app, "GET", "/api/users", Some(bob_token), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "GET", "/api/me", Some(bob_token), None).await.0,
        StatusCode::OK
    );

    // Deactivation blocks login and refresh.
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/api/users/{id}"),
        Some(&admin),
        Some(json!({"is_active": false})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        login(&app, "bob@example.com", "bobs-long-password").await.0,
        StatusCode::UNAUTHORIZED
    );
    let (s, _) = call(
        &app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": bob["refresh_token"]})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // Password reset and role edit.
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/api/users/{id}"),
        Some(&admin),
        Some(json!({
            "is_active": true, "password": "a-brand-new-password", "roles": ["admin", "editor"]
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        login(&app, "bob@example.com", "a-brand-new-password")
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn cannot_remove_last_admin() {
    let app = setup().await;
    let admin = admin_token(&app).await;
    let (_, me) = call(&app, "GET", "/api/me", Some(&admin), None).await;
    let uri = format!("/api/users/{}", me["id"]);
    assert_eq!(
        call(
            &app,
            "PATCH",
            &uri,
            Some(&admin),
            Some(json!({"is_active": false}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    call(
        &app,
        "POST",
        "/api/roles",
        Some(&admin),
        Some(json!({"name": "viewer"})),
    )
    .await;
    assert_eq!(
        call(
            &app,
            "PATCH",
            &uri,
            Some(&admin),
            Some(json!({"roles": ["viewer"]}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn init_admin_refuses_when_users_exist() {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    db::migrate(&pool).await.unwrap();
    db::create_first_admin(&pool, ADMIN_EMAIL, ADMIN_PW)
        .await
        .unwrap();
    assert!(
        db::create_first_admin(&pool, "other@example.com", ADMIN_PW)
            .await
            .is_err()
    );
    assert!(
        db::create_first_admin(&pool, "bad", ADMIN_PW)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn static_files_and_api_404() {
    let app = setup().await;
    let res = app
        .clone()
        .oneshot(Request::get("/login.html").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let (s, body) = call(&app, "GET", "/api/does-not-exist", None, None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn tickets_lifecycle() {
    let app = setup().await;
    let t = admin_token(&app).await;

    let (s, _) = call(&app, "GET", "/api/tickets", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (s, a) = call(
        &app,
        "POST",
        "/api/tickets",
        Some(&t),
        Some(json!({"title": "A"})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(a["key"], "TM-1");
    assert_eq!(a["status"], "backlog");
    assert_eq!(a["priority"], "normal");
    assert_eq!(a["history"].as_array().unwrap().len(), 2);
    let (_, b) = call(
        &app,
        "POST",
        "/api/tickets",
        Some(&t),
        Some(json!({"title": "B", "status": "active"})),
    )
    .await;
    let (s, _) = call(
        &app,
        "POST",
        "/api/tickets",
        Some(&t),
        Some(json!({"title": "  "})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(
        &app,
        "POST",
        "/api/tickets",
        Some(&t),
        Some(json!({"title": "X", "reviewer_id": 999})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (_, board) = call(&app, "GET", "/api/tickets", Some(&t), None).await;
    let order: Vec<_> = board
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["status"].as_str().unwrap())
        .collect();
    assert_eq!(
        order,
        ["completed", "in_review", "active", "up_next", "backlog"]
    );
    assert_eq!(board[2]["tickets"][0]["key"], "TM-2");
    assert_eq!(board[4]["tickets"][0]["key"], "TM-1");

    // move up through the lists, then limits
    let uri = "/api/tickets/1/move";
    for expected in ["up_next", "active", "in_review", "completed"] {
        let (s, r) = call(
            &app,
            "POST",
            uri,
            Some(&t),
            Some(json!({"direction": "up"})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(r["status"], expected);
    }
    let (s, _) = call(
        &app,
        "POST",
        uri,
        Some(&t),
        Some(json!({"direction": "up"})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, r) = call(
        &app,
        "POST",
        uri,
        Some(&t),
        Some(json!({"direction": "down"})),
    )
    .await;
    assert_eq!(
        (s, r["status"].as_str().unwrap()),
        (StatusCode::OK, "in_review")
    );

    // edit, assign, history
    let (s, r) = call(
        &app,
        "PATCH",
        "/api/tickets/2",
        Some(&t),
        Some(json!({"priority": "high", "reviewer_id": 1, "details": "d"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["reviewer"]["id"], 1);
    assert_eq!(r["history"].as_array().unwrap().len(), 3);
    let (_, r) = call(
        &app,
        "PATCH",
        "/api/tickets/2",
        Some(&t),
        Some(json!({"reviewer_id": null})),
    )
    .await;
    assert!(r["reviewer"].is_null());
    assert_eq!(b["id"], 2);

    // archived: read-only except status; paged views
    call(
        &app,
        "PATCH",
        "/api/tickets/2",
        Some(&t),
        Some(json!({"status": "archived"})),
    )
    .await;
    let (s, _) = call(
        &app,
        "PATCH",
        "/api/tickets/2",
        Some(&t),
        Some(json!({"title": "no"})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = call(
        &app,
        "POST",
        "/api/tickets/2/move",
        Some(&t),
        Some(json!({"direction": "up"})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    call(
        &app,
        "PATCH",
        "/api/tickets/1",
        Some(&t),
        Some(json!({"status": "removed"})),
    )
    .await;
    let (_, board) = call(&app, "GET", "/api/tickets", Some(&t), None).await;
    assert!(
        board
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["tickets"].as_array().unwrap().is_empty())
    );
    let (_, p) = call(
        &app,
        "GET",
        "/api/tickets/archived?per_page=1",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(
        (p["total"].as_i64(), p["items"][0]["key"].as_str()),
        (Some(1), Some("TM-2"))
    );
    let (_, p) = call(&app, "GET", "/api/tickets/removed?page=2", Some(&t), None).await;
    assert!(p["items"].as_array().unwrap().is_empty());
    let (s, r) = call(
        &app,
        "PATCH",
        "/api/tickets/2",
        Some(&t),
        Some(json!({"status": "backlog"})),
    )
    .await;
    assert_eq!(
        (s, r["status"].as_str().unwrap()),
        (StatusCode::OK, "backlog")
    );

    let (s, u) = call(&app, "GET", "/api/users/assignable", Some(&t), None).await;
    assert_eq!((s, u.as_array().unwrap().len()), (StatusCode::OK, 1));
    let (s, _) = call(&app, "GET", "/api/tickets/99", Some(&t), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ticket_links_add_remove_and_history() {
    let app = setup().await;
    let t = admin_token(&app).await;
    call(
        &app,
        "POST",
        "/api/tickets",
        Some(&t),
        Some(json!({"title": "A"})),
    )
    .await;
    let uri = "/api/tickets/1/links";

    let (s, r) = call(
        &app,
        "POST",
        uri,
        Some(&t),
        Some(json!({"url": " https://example.com/a?x=1 ", "label": "Docs"})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(r["links"][0]["url"], "https://example.com/a?x=1");
    assert_eq!(r["links"][0]["label"], "Docs");
    let link_id = r["links"][0]["id"].as_i64().unwrap();
    let last = r["history"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["field"], "link_added");
    assert_eq!(last["new_value"], "https://example.com/a?x=1");

    let (s, _) = call(
        &app,
        "POST",
        uri,
        Some(&t),
        Some(json!({"url": "https://example.com/a?x=1"})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    for bad in [
        "javascript:alert(1)",
        "ftp://x.com",
        "http://",
        "https://a b.com",
        "example.com",
    ] {
        let (s, _) = call(&app, "POST", uri, Some(&t), Some(json!({"url": bad}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (s, _) = call(
        &app,
        "POST",
        uri,
        None,
        Some(json!({"url": "https://x.com"})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (s, r) = call(&app, "DELETE", &format!("{uri}/999"), Some(&t), None).await;
    assert_eq!((s, r["links"].is_null()), (StatusCode::NOT_FOUND, true));
    let (s, r) = call(&app, "DELETE", &format!("{uri}/{link_id}"), Some(&t), None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["links"].as_array().unwrap().is_empty());
    let last = r["history"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["field"], "link_removed");
    assert_eq!(last["old_value"], "https://example.com/a?x=1");
    assert!(last["new_value"].is_null());

    // archived tickets are read-only
    call(
        &app,
        "POST",
        uri,
        Some(&t),
        Some(json!({"url": "https://example.com/b"})),
    )
    .await;
    call(
        &app,
        "PATCH",
        "/api/tickets/1",
        Some(&t),
        Some(json!({"status": "archived"})),
    )
    .await;
    let (s, _) = call(
        &app,
        "POST",
        uri,
        Some(&t),
        Some(json!({"url": "https://example.com/c"})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (_, r) = call(&app, "GET", "/api/tickets/1", Some(&t), None).await;
    let id = r["links"][0]["id"].as_i64().unwrap();
    let (s, _) = call(&app, "DELETE", &format!("{uri}/{id}"), Some(&t), None).await;
    assert_eq!(s, StatusCode::CONFLICT);
}
