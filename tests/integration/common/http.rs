//! Driving the real router in-process: build a request, send it through
//! `oneshot`, read the whole response. The suites' own helpers wrap these
//! to return the tuple shape their assertions want.

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::header::{self, AsHeaderName};
use axum::http::{HeaderMap, HeaderName, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// The session cookie's name, as the auth layer sets it.
const SESSION_COOKIE: &str = "mm_session";

/// A request being built. Created by [`request`], sent with
/// [`TestRequest::send`].
pub struct TestRequest {
    method: Method,
    path: String,
    headers: Vec<(HeaderName, String)>,
    body: Option<serde_json::Value>,
}

/// A request for `path`. `method` is a [`Method`] or its name.
pub fn request<M>(method: M, path: &str) -> TestRequest
where
    M: TryInto<Method>,
    M::Error: std::fmt::Debug,
{
    TestRequest {
        method: method.try_into().expect("a valid method"),
        path: path.to_owned(),
        headers: Vec::new(),
        body: None,
    }
}

impl TestRequest {
    /// Signed in as this session token, when there is one.
    pub fn session(self, session: Option<&str>) -> Self {
        match session {
            Some(session) => self.header(header::COOKIE, format!("{SESSION_COOKIE}={session}")),
            None => self,
        }
    }

    pub fn header(mut self, name: HeaderName, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// A JSON body, when there is one.
    pub fn json(mut self, body: Option<serde_json::Value>) -> Self {
        self.body = body;
        self
    }

    pub async fn send(self, app: &Router) -> TestResponse {
        let mut builder = Request::builder().method(self.method).uri(self.path);
        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }
        let body = match self.body {
            Some(json) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = app
            .clone()
            .oneshot(builder.body(body).expect("valid request"))
            .await
            .expect("infallible");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        TestResponse {
            status,
            headers,
            body,
        }
    }
}

/// A response read to the end.
#[derive(Debug)]
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl TestResponse {
    /// The body as JSON; `Null` when it is empty or not JSON.
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn header(&self, name: impl AsHeaderName) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    pub fn location(&self) -> Option<String> {
        self.header(header::LOCATION)
    }
}
