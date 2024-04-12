use std::net::SocketAddr;
use axum_macros::debug_handler;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use axum::{routing::get, Router, Extension, ServiceExt};
use axum::extract::{ConnectInfo, Path};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::handler::Handler;
use axum::response::IntoResponse;
use axum_extra::routing::SpaRouter;
use futures_util::{sink, stream};
// use futures_util::stream::stream::StreamExt;
use futures::{sink::SinkExt, stream::StreamExt};
use shuttle_axum::ShuttleAxum;
use shuttle_runtime::SecretStore;
use sync_wrapper::SyncWrapper;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use varust::{AppState, Msg, NEXT_USERID, Users};

async fn hello_world() -> &'static str {
    "Hello, world!"
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleAxum {

    // We use Secrets.toml to set the BEARER key, just like in a .env file and call it here
    let secret = secrets.get("BEARER").unwrap_or("Bear".to_string());

    // set up router with Secrets & use syncwrapper to make the web service work
    let static_folder = PathBuf::from_str("static").unwrap();
    info!("Serving assets from: {:?}", static_folder.canonicalize());

    let router = build_router(secret, static_folder);
    // let sync_wrapper = SyncWrapper::new(router);

    // Ok(sync_wrapper)
    Ok(router.into())
}

// #[debug_handler]
fn build_router(secret: String, static_folder: PathBuf) -> Router {
    // initialise the Users k/v store and allow the static files to be served
    let users = Users::default();
    let app_state = Arc::new(AppState::default());

    // make an admin route for kicking users
    let admin = Router::new()
        .route("/disconnect/:user_id", get(disconnect_user));
    //     .layer(RequireAuthorizationLayer::bearer(&secret));

    // let static_assets = SpaRouter::new("/", static_folder)
    //     .index_file("index.html");
    // return a new router and nest the admin route into the websocket route
    Router::new()
        .nest_service("/", ServeDir::new(&static_folder).not_found_service(ServeFile::new(static_folder.join("index.html"))))
        // .merge(static_assets)
        .route("/ws", get(ws_handler))
        .nest("/admin", admin)
        .with_state(app_state)
        .layer(Extension(users))
}



// "impl IntoResponse" means we want our function to return a websocket connection
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    // ws.on_upgrade(|socket| handle_socket(socket, state, addr))
    ws.on_upgrade(move |socket| handle_socket(socket))
}

// #[debug_handler]
// async fn handle_socket(stream: WebSocket, state: Users, addr: SocketAddr) {
async fn handle_socket(stream: WebSocket) {
    // When a new user enters the chat (opens the websocket connection), assign them a user ID
    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // By splitting the websocket into a receiver and sender, we can send and receive at the same time.
    let (mut websocket_sender, mut websocket_receiver) = stream.split();

    // Create a new channel for async task management (stored in Users hashmap)
    let (app_sender, mut app_receiver): (UnboundedSender<Message>, UnboundedReceiver<Message>) = mpsc::unbounded_channel();

// If a message has been received, send the message (expect on error)
    tokio::spawn(async move {
        while let Some(msg) = app_receiver.recv().await {
            info!("Publish to all: {:?}", msg);
            websocket_sender.send(msg).await.expect("Error while sending message");
        }
        websocket_sender.close().await.unwrap();
    });

    // if there's a message and the message is OK, broadcast it along all available open websocket connections
    while let Some(Ok(result)) = websocket_receiver.next().await {
        println!("{:?}", result);
        if let Ok(result) = enrich_result(result, my_id) {
            info!("Received from client: {:?}", result);
            // broadcast_msg(result, &state).await;
            println!("Hello");
        }
    }
}


async fn broadcast_msg(msg: Message, state: Arc<AppState>) {
// "If let" is basically a simple match statement, which is perfect for this use case
// as we want to only match against one condition.
    if let Message::Text(msg) = msg {
        for (&_uid, tx) in state.users.lock().await.iter() {
            tx.send(Message::Text(msg.clone()))
                .expect("Failed to send Message")
        }
    }
}

fn enrich_result(result: Message, id: usize) -> Result<Message, serde_json::Error> {
    match result {
        Message::Text(msg) => {
            let mut msg: Msg = serde_json::from_str(&msg)?;
            msg.uid = Some(id);
            let msg = serde_json::to_string(&msg)?;
            Ok(Message::Text(msg))
        }
        _ => Ok(result),
    }
}

async fn disconnect_user(
    Path(user_id): Path<usize>,
    Extension(users): Extension<Users>,
) -> impl IntoResponse {
    // disconnect(user_id, &users).await;
    "Done"
}

