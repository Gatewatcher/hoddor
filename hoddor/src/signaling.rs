use crate::console;
use futures_channel::mpsc;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use js_sys::Function;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SignalingMessage {
    Join {
        peer_id: String,
    },
    Offer {
        from: String,
        to: String,
        sdp: String,
    },
    Answer {
        from: String,
        to: String,
        sdp: String,
    },
    IceCandidate {
        from: String,
        to: String,
        candidate: String,
    },
    Leave {
        peer_id: String,
    },
    Discovery {
        from: String,
    },
}

pub struct SignalingClient {
    ws: WebSocket,
    peer_id: String,
    #[allow(dead_code)]
    onmessage_callback: Function,
    #[allow(dead_code)]
    onerror_callback: Function,
}

impl SignalingClient {
    pub fn send_offer(&self, to: String, sdp: String) -> Result<(), JsValue> {
        let offer_msg = SignalingMessage::Offer {
            from: self.peer_id.clone(),
            to: to.clone(),
            sdp,
        };
        let msg_str = serde_json::to_string(&offer_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize message: {}", e)))?;
        console::log(&format!(
            "Sending offer from {} to {}: {}",
            self.peer_id, to, msg_str
        ));
        self.ws.send_with_str(&msg_str)?;
        Ok(())
    }

    pub fn send_answer(&self, to: String, sdp: String) -> Result<(), JsValue> {
        let answer_msg = SignalingMessage::Answer {
            from: self.peer_id.clone(),
            to: to.clone(),
            sdp,
        };
        let msg_str = serde_json::to_string(&answer_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize message: {}", e)))?;
        console::log(&format!(
            "Sending answer from {} to {}: {}",
            self.peer_id, to, msg_str
        ));
        self.ws.send_with_str(&msg_str)?;
        Ok(())
    }

    pub fn send_ice_candidate(&self, to: String, candidate: String) -> Result<(), JsValue> {
        let ice_msg = SignalingMessage::IceCandidate {
            from: self.peer_id.clone(),
            to: to.clone(),
            candidate,
        };
        let msg_str = serde_json::to_string(&ice_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize message: {}", e)))?;
        console::log(&format!(
            "Sending ICE candidate from {} to {}: {}",
            self.peer_id, to, msg_str
        ));
        self.ws.send_with_str(&msg_str)?;
        Ok(())
    }

    pub fn set_message_handler(&mut self, sender: UnboundedSender<SignalingMessage>) {
        let peer_id = self.peer_id.clone();

        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let text_str = String::from(text);
                console::log(&format!("Received message: {}", text_str));

                match serde_json::from_str::<SignalingMessage>(&text_str) {
                    Ok(msg) => {
                        // Check if this message is for us
                        let is_for_us = match &msg {
                            SignalingMessage::Offer { to, .. } => to == &peer_id,
                            SignalingMessage::Answer { to, .. } => to == &peer_id,
                            SignalingMessage::IceCandidate { to, .. } => to == &peer_id,
                            SignalingMessage::Join { .. } => true,
                            SignalingMessage::Leave { .. } => true,
                            SignalingMessage::Discovery { .. } => true,
                        };

                        if is_for_us {
                            console::log(&format!("Processing message for {}: {:?}", peer_id, msg));
                            match sender.unbounded_send(msg) {
                                Ok(_) => (),
                                Err(e) => {
                                    if e.is_disconnected() {
                                        console::log(&format!(
                                            "Message channel disconnected for {}, ignoring message",
                                            peer_id
                                        ));
                                    } else {
                                        console::error(&format!(
                                            "Failed to forward message: {:?}",
                                            e
                                        ));
                                    }
                                }
                            }
                        } else {
                            console::log("Message not for us, ignoring");
                        }
                    }
                    Err(e) => console::error(&format!("Failed to parse message: {:?}", e)),
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        self.ws
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        self.onmessage_callback = onmessage_callback.into_js_value().unchecked_into();
    }

    pub fn get_websocket(&self) -> &WebSocket {
        &self.ws
    }

    pub fn set_onopen<T>(&self, callback: &T) -> Result<(), JsValue>
    where
        T: wasm_bindgen::JsCast + ?Sized,
    {
        self.ws.set_onopen(Some(callback.unchecked_ref()));
        Ok(())
    }

    pub fn set_onerror<T>(&self, callback: &T) -> Result<(), JsValue>
    where
        T: wasm_bindgen::JsCast + ?Sized,
    {
        self.ws.set_onerror(Some(callback.unchecked_ref()));
        Ok(())
    }

    pub fn new(server_url: &str, peer_id: String) -> Result<Rc<RefCell<Self>>, JsValue> {
        console::log(&format!(
            "Creating new WebSocket connection to {}",
            server_url
        ));
        let ws = WebSocket::new(server_url)?;

        // Set up error handler with more detailed logging
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            console::error(&format!("WebSocket error: {:?}", e));
            // Try to log more error details if available
            if let Ok(err_details) = js_sys::Reflect::get(&e, &"error".into()) {
                console::error(&format!("Error details: {:?}", err_details));
            }
        }) as Box<dyn FnMut(ErrorEvent)>)
        .into_js_value();

        ws.set_onerror(Some(onerror_callback.unchecked_ref()));

        console::log(&format!("WebSocket setup complete for peer {}", peer_id));

        let empty_callback =
            Closure::wrap(Box::new(move |_: MessageEvent| {}) as Box<dyn FnMut(MessageEvent)>)
                .into_js_value();

        Ok(Rc::new(RefCell::new(Self {
            ws,
            peer_id,
            onmessage_callback: empty_callback.unchecked_into(),
            onerror_callback: onerror_callback.unchecked_into(),
        })))
    }
}

pub struct SignalingManager {
    clients: RefCell<Vec<Rc<RefCell<SignalingClient>>>>,
}

impl Default for SignalingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalingManager {
    pub fn new() -> Self {
        SignalingManager {
            clients: RefCell::new(Vec::new()),
        }
    }

    pub fn cleanup_client(&self, peer_id: &str) {
        let mut clients = self.clients.borrow_mut();
        clients.retain(|client| client.borrow().peer_id != peer_id);
    }

    pub fn send_offer(&self, to_peer_id: String, sdp: String) -> Result<(), JsValue> {
        let clients = self.clients.borrow();
        if let Some(client) = clients.first() {
            let client = client.borrow();
            console::log(&format!(
                "SignalingManager: Sending offer from {} to {}",
                client.peer_id, to_peer_id
            ));
            client.send_offer(to_peer_id, sdp)?;
        } else {
            console::error("No local client found to send offer");
        }
        Ok(())
    }

    pub fn send_answer(&self, to_peer_id: String, sdp: String) -> Result<(), JsValue> {
        let clients = self.clients.borrow();
        if let Some(client) = clients.first() {
            let client = client.borrow();
            console::log(&format!(
                "SignalingManager: Sending answer from {} to {}",
                client.peer_id, to_peer_id
            ));
            client.send_answer(to_peer_id, sdp)?;
        } else {
            console::error("No local client found to send answer");
        }
        Ok(())
    }

    pub fn send_ice_candidate(&self, to_peer_id: String, candidate: String) -> Result<(), JsValue> {
        let clients = self.clients.borrow();
        if let Some(client) = clients.first() {
            let client = client.borrow();
            console::log(&format!(
                "SignalingManager: Sending ICE candidate from {} to {}",
                client.peer_id, to_peer_id
            ));
            client.send_ice_candidate(to_peer_id, candidate)?;
        } else {
            console::error("No local client found to send ICE candidate");
        }
        Ok(())
    }

    pub fn get_client(&self, peer_id: &str) -> Option<Rc<RefCell<SignalingClient>>> {
        self.clients
            .borrow()
            .iter()
            .find(|client| client.borrow().peer_id == peer_id)
            .cloned()
    }

    pub fn add_client(
        &self,
        server_url: &str,
        peer_id: String,
    ) -> Result<UnboundedReceiver<SignalingMessage>, JsValue> {
        if let Some(existing_client) = self.get_client(&peer_id) {
            let (sender, receiver) = mpsc::unbounded::<SignalingMessage>();
            {
                let mut client_ref = existing_client.borrow_mut();
                client_ref.set_message_handler(sender);

                if client_ref.get_websocket().ready_state() == web_sys::WebSocket::OPEN {
                    let join_msg = SignalingMessage::Join {
                        peer_id: peer_id.clone(),
                    };
                    if let Ok(msg_str) = serde_json::to_string(&join_msg) {
                        console::log(&format!(
                            "Sending join message on existing connection: {}",
                            msg_str
                        ));
                        if let Err(e) = client_ref.get_websocket().send_with_str(&msg_str) {
                            console::error(&format!("Failed to send join message: {:?}", e));
                        }
                    }
                }
            }
            return Ok(receiver);
        }

        let (sender, receiver) = mpsc::unbounded::<SignalingMessage>();
        let client = SignalingClient::new(server_url, peer_id.clone())?;

        {
            let mut client_ref = client.borrow_mut();
            client_ref.set_message_handler(sender.clone());
        }

        self.clients.borrow_mut().push(client);
        console::log(&format!("Added new signaling client for peer {}", peer_id));

        Ok(receiver)
    }
}

thread_local! {
    static SIGNALING_MANAGER: RefCell<SignalingManager> = RefCell::new(SignalingManager::new());
}

pub fn with_signaling_manager<F, R>(f: F) -> R
where
    F: FnOnce(&SignalingManager) -> R,
{
    SIGNALING_MANAGER.with(|manager| f(&manager.borrow()))
}
