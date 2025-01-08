use crate::messages::SignalingMessage;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    web, App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_ws::{self, Message};
use env_logger::Env;
use futures::StreamExt;
use log::{debug, error, info};
use serde_json::json;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

mod config;
mod messages;
mod security;

use config::CONFIG;
use security::{generate_token, get_client_ip, validate_origin, verify_token, RateLimiter};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct PeerState {
    session: actix_ws::Session,
    peer_id: String,
    last_seen: std::time::Instant,
}

impl fmt::Debug for PeerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeerState")
            .field("peer_id", &self.peer_id)
            .field("last_seen", &self.last_seen)
            .finish()
    }
}

impl PeerState {
    fn new(session: actix_ws::Session, peer_id: String) -> Self {
        Self {
            session,
            peer_id,
            last_seen: std::time::Instant::now(),
        }
    }

    fn update_session(&mut self, session: actix_ws::Session) {
        self.session = session;
        self.last_seen = std::time::Instant::now();
    }

    fn is_alive(&self) -> bool {
        self.last_seen.elapsed() < std::time::Duration::from_secs(60)
    }

    fn touch(&mut self) {
        self.last_seen = std::time::Instant::now();
    }
}

struct AppState {
    peers: Arc<Mutex<HashMap<String, PeerState>>>,
}

async fn forward_message(peers: &Arc<Mutex<HashMap<String, PeerState>>>, to: &str, msg: &str) {
    let mut peers_lock = peers.lock().await;

    if let Some(peer) = peers_lock.get_mut(to) {
        debug!("Found peer {}, forwarding message: {}", to, msg);
        peer.touch(); // Update last seen time
                      // Clone the session to avoid borrow checker issues
        let mut session = peer.session.clone();
        // Drop the lock before awaiting
        drop(peers_lock);

        match session.text(msg.to_string()).await {
            Ok(_) => debug!("Successfully forwarded message to {}", to),
            Err(e) => error!("Failed to forward message to {}: {:?}", to, e),
        }
    } else {
        debug!("Peer {} not found", to);
    }
}

async fn handle_signaling_message(
    msg: SignalingMessage,
    session: &mut actix_ws::Session,
    peers: &Arc<Mutex<HashMap<String, PeerState>>>,
    client_ip: &str,
) -> Result<(), Error> {
    match msg {
        SignalingMessage::Join { peer_id } => {
            info!("Peer {} joined from IP {}", peer_id, client_ip);

            let response = SignalingMessage::Join {
                peer_id: peer_id.clone(),
            };
            if let Ok(msg_str) = serde_json::to_string(&response) {
                debug!("Sending join confirmation: {}", msg_str);
                if let Err(e) = session.text(msg_str.clone()).await {
                    error!("Failed to send join confirmation: {:?}", e);
                    return Err(ErrorInternalServerError("Failed to send join confirmation"));
                }
            }

            {
                let mut peers_lock = peers.lock().await;
                if let Some(existing_peer) = peers_lock.get_mut(&peer_id) {
                    existing_peer.update_session(session.clone());
                    info!("Updated existing peer {}", peer_id);
                } else {
                    peers_lock.insert(
                        peer_id.clone(),
                        PeerState::new(session.clone(), peer_id.clone()),
                    );
                    info!("Added new peer {}", peer_id);
                }
            }

            // Now notify other peers about the new peer
            let discovery_msg = SignalingMessage::Discovery {
                from: peer_id.clone(),
            };
            if let Ok(discovery_str) = serde_json::to_string(&discovery_msg) {
                let peer_ids = {
                    let peers_lock = peers.lock().await;
                    peers_lock.keys().cloned().collect::<Vec<String>>()
                };

                debug!("Current peers: {:?}", peer_ids);
                for other_peer_id in peer_ids {
                    if other_peer_id != peer_id {
                        debug!(
                            "Notifying peer {} about new peer {}",
                            other_peer_id, peer_id
                        );
                        forward_message(peers, &other_peer_id, &discovery_str).await;
                    }
                }
            }
        }
        SignalingMessage::Leave { peer_id } => {
            info!("Peer {} left", peer_id);
            let mut peers_lock = peers.lock().await;
            peers_lock.remove(&peer_id);
            let peer_ids: Vec<String> = peers_lock.keys().cloned().collect();
            debug!("Current peers after leave: {:?}", peer_ids);
        }
        SignalingMessage::Offer { from, to, sdp } => {
            debug!("Offer from {} to {}", from, to);
            let msg_str = serde_json::to_string(&SignalingMessage::Offer {
                from: from.clone(),
                to: to.clone(),
                sdp: sdp.clone(),
            })?;
            debug!("Forwarding offer: {}", msg_str);
            forward_message(peers, &to, &msg_str).await;
        }
        SignalingMessage::Answer { from, to, sdp } => {
            debug!("Answer from {} to {}", from, to);
            let msg_str = serde_json::to_string(&SignalingMessage::Answer {
                from: from.clone(),
                to: to.clone(),
                sdp: sdp.clone(),
            })?;
            debug!("Forwarding answer: {}", msg_str);
            forward_message(peers, &to, &msg_str).await;
        }
        SignalingMessage::IceCandidate {
            from,
            to,
            candidate,
        } => {
            debug!("ICE candidate from {} to {}", from, to);
            let msg_str = serde_json::to_string(&SignalingMessage::IceCandidate {
                from: from.clone(),
                to: to.clone(),
                candidate: candidate.clone(),
            })?;
            debug!("Forwarding ICE candidate: {}", msg_str);
            forward_message(peers, &to, &msg_str).await;
        }
        SignalingMessage::Discovery { from } => {
            debug!("Discovery from {}", from);
            // Handle discovery messages
        }
    }
    Ok(())
}

async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    rate_limiter: web::Data<RateLimiter>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let client_ip = get_client_ip(&req);

    let token = req
        .query_string()
        .split('&')
        .find(|pair| pair.starts_with("token="))
        .and_then(|pair| pair.split('=').nth(1))
        .ok_or_else(|| ErrorUnauthorized("Missing token in query parameters"))?;

    verify_token(token, &CONFIG.jwt_secret)?;

    if !rate_limiter.check_rate_limit(&client_ip) {
        error!("Rate limit exceeded for IP: {}", client_ip);
        return Ok(HttpResponse::TooManyRequests().finish());
    }

    match validate_origin(&req) {
        Ok(_) => (),
        Err(_) => {
            error!("Invalid origin from IP: {}", client_ip);
            return Ok(HttpResponse::Forbidden().finish());
        }
    }

    match actix_ws::handle(&req, body) {
        Ok((response, session, mut msg_stream)) => {
            info!("WebSocket connection established from {}", client_ip);

            let peers = app_state.peers.clone();
            actix_web::rt::spawn(async move {
                let mut ws = session;
                let mut last_heartbeat = std::time::Instant::now();
                let mut interval = interval(HEARTBEAT_INTERVAL);

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if std::time::Instant::now().duration_since(last_heartbeat) > HEARTBEAT_INTERVAL * 2 {
                                info!("Client heartbeat missed, disconnecting...");
                                break;
                            }

                            if let Err(e) = ws.ping(b"").await {
                                error!("Failed to send ping: {:?}", e);
                                break;
                            }
                        }

                        Some(msg) = msg_stream.next() => {
                            match msg {
                                Ok(msg) => {
                                    match msg {
                                        Message::Text(text) => {
                                            match serde_json::from_str::<SignalingMessage>(&text) {
                                                Ok(msg) => {
                                                    if let Err(e) = handle_signaling_message(msg, &mut ws, &peers, &client_ip).await {
                                                        error!("Error handling message: {:?}", e);
                                                    }
                                                }
                                                Err(e) => error!("Failed to parse message: {:?}", e),
                                            }
                                        }
                                        Message::Ping(bytes) => {
                                            last_heartbeat = std::time::Instant::now();
                                            if let Err(e) = ws.pong(&bytes).await {
                                                error!("Failed to send pong: {:?}", e);
                                                break;
                                            }
                                        }
                                        Message::Pong(_) => {
                                            last_heartbeat = std::time::Instant::now();
                                        }
                                        Message::Close(reason) => {
                                            info!("Client disconnected: {:?}", reason);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                Err(e) => {
                                    error!("Error reading message: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                }

                info!("WebSocket connection closed for {}", client_ip);
            });

            Ok(response)
        }
        Err(e) => {
            error!("Failed to establish WebSocket connection: {:?}", e);
            Err(e)
        }
    }
}

async fn generate_auth_token() -> Result<HttpResponse, Error> {
    let token = generate_token(&CONFIG.jwt_secret)?;
    Ok(HttpResponse::Ok().json(json!({ "token": token })))
}

async fn cleanup_stale_peers(peers: Arc<Mutex<HashMap<String, PeerState>>>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        interval.tick().await;

        let mut peers_lock = peers.lock().await;

        // Remove peers that haven't been seen in the last 60 seconds
        let stale_peers: Vec<String> = peers_lock
            .iter()
            .filter(|(_, state)| !state.is_alive())
            .map(|(id, _)| id.clone())
            .collect();

        for peer_id in stale_peers.iter() {
            if let Some(peer) = peers_lock.get(peer_id.as_str()) {
                let session = peer.session.clone();
                if let Err(e) = session.close(None).await {
                    error!("Failed to close connection for peer {}: {:?}", peer_id, e);
                }
            }
            peers_lock.remove(peer_id.as_str());
            debug!("Removing stale peer: {}", peer_id);
        }

        if !stale_peers.is_empty() {
            let remaining_peers: Vec<String> = peers_lock.keys().cloned().collect();
            info!(
                "Cleaned up {} stale peers. Remaining peers: {:?}",
                stale_peers.len(),
                remaining_peers
            );
        }

        drop(peers_lock);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    let rate_limiter = web::Data::new(RateLimiter::new(60, 100)); // 100 requests per minute
    let app_state = web::Data::new(AppState {
        peers: Arc::new(Mutex::new(HashMap::new())),
    });

    let peers_for_cleanup = app_state.peers.clone();

    tokio::spawn(async move {
        cleanup_stale_peers(peers_for_cleanup).await;
    });

    let bind_addr = format!("0.0.0.0:{}", CONFIG.port);
    info!("Starting signaling server on {}", bind_addr);

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::clone(&rate_limiter))
            .app_data(web::Data::clone(&app_state))
            .route(
                "/",
                web::get().to(|| async {
                    info!("Received request to root endpoint");
                    HttpResponse::Ok().body("Server is running")
                }),
            )
            .route("/ws", web::get().to(ws_handler))
            .route("/token", web::post().to(generate_auth_token))
    })
    .bind(&bind_addr)?
    .workers(4)
    .max_connections(CONFIG.max_connections)
    .run();

    let srv = server.handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Received shutdown signal");
        srv.stop(true).await;
    });

    server.await
}
