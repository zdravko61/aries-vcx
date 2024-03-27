use std::sync::Arc;

use aries_vcx_core::wallet::base_wallet::BaseWallet;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::header::{HeaderMap, ACCEPT},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use messages::AriesMessage;
use serde_json::{json, Value};

use crate::{
    aries_agent::{Agent, ArcAgent},
    didcomm_handlers,
    persistence::MediatorPersistence,
};

pub async fn oob_invite_qr(
    headers: HeaderMap,
    State(agent): State<ArcAgent<impl BaseWallet, impl MediatorPersistence>>,
) -> Response {
    let Json(oob_json) = oob_invite_json(State(agent)).await;
    let preferred_mimetype = headers
        .get(ACCEPT)
        .map(|s| s.to_str().unwrap_or_default())
        .unwrap_or_default();
    match preferred_mimetype {
        "application/json" => Json(oob_json).into_response(),
        _ => {
            let oob_string = serde_json::to_string_pretty(&oob_json).unwrap();
            let qr = fast_qr::QRBuilder::new(oob_string.clone()).build().unwrap();
            let oob_qr_svg = fast_qr::convert::svg::SvgBuilder::default().to_str(&qr);
            Html(format!(
                "<style>
                        svg {{
                            width: 50%;
                            height: 50%;
                        }}
                    </style>
                    {oob_qr_svg} <br>
                    <pre>{oob_string}</pre>"
            ))
            .into_response()
        }
    }
}
pub async fn oob_invite_base64url(
    State(agent): State<ArcAgent<impl BaseWallet, impl MediatorPersistence>>,
) -> Json<Value> {
    let oob = agent.get_oob_invite().unwrap();
    let msg = AriesMessage::from(oob);

    let url = base64_url::encode(&msg.to_string());

    let mut endpoint = agent.get_service_ref().unwrap().service_endpoint.clone();
    endpoint.set_path("");

    Json(json!({ "invitationUrl": format!("{}?oob={}", endpoint, url)}))
}

pub async fn oob_invite_json(
    State(agent): State<ArcAgent<impl BaseWallet, impl MediatorPersistence>>,
) -> Json<Value> {
    let oob = agent.get_oob_invite().unwrap();
    let msg = AriesMessage::from(oob);
    Json(serde_json::to_value(msg).unwrap())
}

pub async fn handle_didcomm(
    State(agent): State<ArcAgent<impl BaseWallet, impl MediatorPersistence>>,
    didcomm_msg: Bytes,
) -> Result<Json<Value>, String> {
    didcomm_handlers::handle_aries(State(agent), didcomm_msg).await
}

pub async fn readme() -> Html<String> {
    Html("<p>Please refer to the API section of <a>readme</a> for usage. Thanks. </p>".into())
}

pub async fn build_router(
    agent: Agent<impl BaseWallet + 'static, impl MediatorPersistence>,
) -> Router {
    Router::default()
        .route("/", get(readme))
        .route("/register", get(oob_invite_qr))
        .route("/invite", get(oob_invite_base64url))
        .route("/register.json", get(oob_invite_json))
        .route("/didcomm", post(handle_didcomm))
        //FIXME: why this does not work all the time ??
        .layer(DefaultBodyLimit::max(1024 * 1024 * 30))
        .layer(tower_http::catch_panic::CatchPanicLayer::new())
        .with_state(Arc::new(agent))
}
