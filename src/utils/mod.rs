use std::collections::HashMap;

use bytes::Bytes;
use http::{header, Response, StatusCode};
use pingora::{http::ResponseHeader, prelude::*};

pub async fn send_response(
    session: &mut Session,
    status: StatusCode,
    content_type: Option<&'static str>,
    body: Option<Bytes>,
    headers: Option<HashMap<String, String>>,
) -> Result<()> {
    let cl = body.as_ref().map(|b| b.len()).unwrap_or(0);
    let mut bd = Response::builder()
        .status(status)
        .header(header::CONTENT_LENGTH, cl);
    if let Some(headers) = headers {
        for (key, value) in headers {
            bd = bd.header(key, value);
        }
    }
    if let Some(content_type) = content_type {
        bd = bd.header(header::CONTENT_TYPE, content_type);
    } else {
        bd = bd.header(header::CONTENT_TYPE, "text/plain");
    }

    if let Some(body) = body {
        let resp = bd.body(body).unwrap();
        let (parts, body) = resp.into_parts();
        let resp_header: ResponseHeader = parts.into();
        session
            .write_response_header(Box::new(resp_header), false)
            .await?;
        session.write_response_body(Some(body), true).await
    } else {
        let resp = bd.body(()).unwrap();
        let (parts, _) = resp.into_parts();
        let resp_header: ResponseHeader = parts.into();
        session
            .write_response_header(Box::new(resp_header), false)
            .await?;
        session.write_response_body(None, true).await
    }
}
