//! 验证上下游兼容规则经实际请求编码和 HTTP/WS 传输生效

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use provider_openai::transport::protocol::responses::CodexResponsesRequest;
use provider_openai::transport::{CodexBackendClient, CodexRequestContext, CodexWebSocketPool};
use serde_json::{Map, Value, json};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::tungstenite::Message;

use crate::transport::{
    CodexBackendClientTestExt, accept_codex_test_websocket, accept_codex_test_websocket_with,
    completed_websocket_response, read_http_request_with_body, request_context, test_wire_profile,
    write_completed_sse_response,
};

mod headers;
mod request;
