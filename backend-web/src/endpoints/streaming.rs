use crate::auth::Claims;
use crate::error::WebError;
use crate::types::{AppState, RunningTaskState, TaskId, TeamId};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use snafu::{location, IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::time::Duration;
use tokio::{select, time};
use tracing::{debug, info, instrument};

#[derive(Debug, Snafu)]
enum WebsocketError {
    #[snafu(display("Client sent no hello at {location}"))]
    NoClientHello {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Client hello read failed at {location}"))]
    ClientHelloRead {
        source: axum::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Client hello could not be converted to text at {location}"))]
    ClientHelloNoText {
        source: axum::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Client claims not valid at {location}"))]
    ClaimsNotValid {
        source: WebError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to send initial data at {location}"))]
    InitialDataSend {
        source: axum::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to send update at {location}"))]
    UpdateSend {
        source: axum::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("No claims sent within timeout"))]
    NoClaimsSent { source: time::error::Elapsed },
}

#[instrument(skip_all)]
pub async fn head_running_task_info(
    State(app_state): State<AppState>,
    Path(task_id): Path<TaskId>,
) -> Result<(), WebError> {
    if app_state
        .executor
        .lock()
        .unwrap()
        .get_running_task(&task_id)
        .is_some()
    {
        Ok(())
    } else {
        Err(WebError::not_found(location!()))
    }
}

#[instrument(skip_all)]
pub async fn get_running_task_info(
    State(app_state): State<AppState>,
    Path(task_id): Path<TaskId>,
    ws: WebSocketUpgrade,
) -> Result<Response, WebError> {
    let Some(state) = app_state
        .executor
        .lock()
        .unwrap()
        .get_running_task(&task_id)
    else {
        return Err(WebError::not_found(location!()));
    };

    Ok(ws.on_upgrade(|ws| async move {
        let (mut ws_write, ws_read) = ws.split();
        let res = handle_websocket(&app_state, state, ws_read, &mut ws_write).await;
        if let Err(e) = res {
            debug!(error = %Report::from_error(&e), "Error handling websocket");
            let _ = ws_write
                .send(Message::Text(
                    serde_json::to_string(&json!({"error": e.to_string() })).unwrap(),
                ))
                .await;
        }
    }))
}

#[instrument(skip_all)]
async fn handle_websocket(
    app_state: &AppState,
    state: RunningTaskState,
    mut ws_read: SplitStream<WebSocket>,
    ws_write: &mut SplitSink<WebSocket, Message>,
) -> Result<(), WebsocketError> {
    // Get greeted
    let _claims = time::timeout(
        Duration::from_secs(10),
        read_client_claims(app_state, &mut ws_read),
    )
    .await
    .context(NoClaimsSentSnafu)??;

    select! {
        _ = answer_pings(ws_read) => Ok(()),
        e = stream_events(state, ws_write) => e
    }
}

#[instrument(skip_all)]
async fn answer_pings(mut ws_read: SplitStream<WebSocket>) {
    while let Some(res) = ws_read.next().await {
        match res {
            Ok(_) => {}
            Err(e) => {
                info!("Websocket read error: {e:?}");
                return;
            }
        }
    }
}

async fn read_client_claims(
    state: &AppState,
    ws_read: &mut SplitStream<WebSocket>,
) -> Result<Claims, WebsocketError> {
    let client_hello = ws_read
        .next()
        .await
        .ok_or("client sent no hello")
        .map_err(|_| NoClientHelloSnafu {}.into_error(NoneError))?
        .context(ClientHelloReadSnafu)?;
    let token = client_hello.into_text().context(ClientHelloNoTextSnafu)?;
    let claims = Claims::<TeamId>::from_token(state, &token)
        .await
        .context(ClaimsNotValidSnafu)?;

    Ok(claims)
}

#[instrument(skip_all)]
async fn stream_events(
    mut state: RunningTaskState,
    write: &mut SplitSink<WebSocket, Message>,
) -> Result<(), WebsocketError> {
    for update in state.so_far {
        write
            .send(Message::Text(serde_json::to_string(&update).unwrap()))
            .await
            .context(InitialDataSendSnafu)?;
    }

    while let Ok(update) = state.receiver.recv().await {
        write
            .send(Message::Text(serde_json::to_string(&update).unwrap()))
            .await
            .context(UpdateSendSnafu)?;
    }

    Ok(())
}
