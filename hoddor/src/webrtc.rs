use crate::console;
use crate::signaling::{with_signaling_manager, SignalingMessage};
use crate::sync::SyncMessage;
use crate::vault::update_vault_from_sync;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use futures_channel::mpsc;
use js_sys::{Array, JsString, Object, Reflect};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    ErrorEvent, MessageEvent, RtcConfiguration, RtcDataChannel, RtcIceCandidate,
    RtcIceCandidateInit, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit,
};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebRtcMetadata {
    pub peer_id: String,
    pub permissions: HashMap<String, AccessLevel>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum AccessLevel {
    Viewer,
    Contributor,
    Administrator,
}

#[derive(Debug, Clone)]
pub struct WebRtcPeer {
    metadata: WebRtcMetadata,
    connection: RtcPeerConnection,
    data_channel: Option<RtcDataChannel>,
    remote_peer_id: Option<String>,
    connected: Rc<RefCell<bool>>,
    channel_open: Rc<RefCell<bool>>,
    ice_connected: Rc<RefCell<bool>>,
    message_sender: UnboundedSender<Vec<u8>>,
    connection_state_sender: UnboundedSender<bool>,
    is_offerer: bool,
}

impl WebRtcPeer {
    pub fn metadata(&self) -> &WebRtcMetadata {
        &self.metadata
    }

    pub fn remote_peer_id(&self) -> Option<String> {
        self.remote_peer_id.clone()
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.borrow()
    }

    pub fn set_connected(&mut self, connected: bool) {
        *self.connected.borrow_mut() = connected;
        let _ = self.connection_state_sender.unbounded_send(connected);
    }

    pub fn is_channel_open(&self) -> bool {
        *self.channel_open.borrow()
    }

    pub fn is_ice_connected(&self) -> bool {
        *self.ice_connected.borrow()
    }

    pub fn is_ready(&self) -> bool {
        let connected = *self.connected.borrow();
        let channel_open = *self.channel_open.borrow();
        let ice_connected = *self.ice_connected.borrow();

        let ready = connected && channel_open && ice_connected;

        console::log(&format!("Checking connection readiness: connected={}, channel_open={}, ice_connected={}, ready={}",
            connected, channel_open, ice_connected, ready));

        ready
    }

    pub async fn create_peer(
        peer_id: String,
        stun_servers: Vec<String>,
    ) -> Result<(Self, UnboundedReceiver<Vec<u8>>), JsValue> {
        let rtc_config = RtcConfiguration::new();
        let ice_servers = Array::new();

        for server in stun_servers {
            let server_dict = Object::new();
            Reflect::set(&server_dict, &"urls".into(), &server.into())?;
            ice_servers.push(&server_dict);
        }

        rtc_config.set_ice_servers(&ice_servers);

        let connection = RtcPeerConnection::new_with_configuration(&rtc_config)?;

        let (sender, receiver) = mpsc::unbounded();
        let (connection_state_sender, _) = mpsc::unbounded();

        let channel_open = Rc::new(RefCell::new(false));
        let metadata = WebRtcMetadata {
            peer_id: peer_id.clone(),
            permissions: HashMap::new(),
        };

        let ice_connected = Rc::new(RefCell::new(false));

        let mut peer = Self {
            metadata,
            connection,
            data_channel: None,
            remote_peer_id: None,
            connected: Rc::new(RefCell::new(false)),
            channel_open,
            ice_connected,
            message_sender: sender,
            connection_state_sender,
            is_offerer: false,
        };

        peer.setup_connection().await?;

        Ok((peer, receiver))
    }

    async fn setup_connection(&mut self) -> Result<(), JsValue> {
        console::log("Setting up WebRTC connection handlers...");

        let connected_flag = Rc::new(RefCell::new(false));
        let connected_flag_clone = connected_flag.clone();
        let connection_ref = self.connection.clone();
        let connection_ref2 = self.connection.clone();
        let connection_ref3 = self.connection.clone();
        let state_sender = self.connection_state_sender.clone();

        let onicegatheringstatechange_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let state = connection_ref.ice_gathering_state();
            console::log(&format!("ICE gathering state changed to: {:?}", state));

            match state {
                web_sys::RtcIceGatheringState::New => {
                    console::log("ICE gathering starting...");
                }
                web_sys::RtcIceGatheringState::Gathering => {
                    console::log("ICE gathering in progress...");
                }
                web_sys::RtcIceGatheringState::Complete => {
                    console::log("ICE gathering complete");
                }
                _ => {
                    console::warn("Unknown ICE gathering state");
                }
            }
        })
            as Box<dyn FnMut(web_sys::Event)>);

        self.connection.set_onicegatheringstatechange(Some(
            onicegatheringstatechange_callback.as_ref().unchecked_ref(),
        ));
        onicegatheringstatechange_callback.forget();

        let onconnectionstatechange_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let state = connection_ref2.connection_state();
            let is_connected = state == web_sys::RtcPeerConnectionState::Connected;
            *connected_flag_clone.borrow_mut() = is_connected;
            let _ = state_sender.unbounded_send(is_connected);

            console::log(&format!(
                "Connection state changed to: {:?}, connected={}",
                state, is_connected
            ));

            match state {
                web_sys::RtcPeerConnectionState::New => {
                    console::log("Connection is new");
                }
                web_sys::RtcPeerConnectionState::Connecting => {
                    console::log("Connection is establishing...");
                }
                web_sys::RtcPeerConnectionState::Connected => {
                    console::log("Connection established!");
                    *connected_flag_clone.borrow_mut() = true;
                    let _ = state_sender.unbounded_send(true);
                }
                web_sys::RtcPeerConnectionState::Disconnected => {
                    console::log("Connection disconnected");
                    *connected_flag_clone.borrow_mut() = false;
                    let _ = state_sender.unbounded_send(false);
                }
                web_sys::RtcPeerConnectionState::Failed => {
                    console::log("Connection failed");
                    *connected_flag_clone.borrow_mut() = false;
                    let _ = state_sender.unbounded_send(false);
                }
                web_sys::RtcPeerConnectionState::Closed => {
                    console::log("Connection closed");
                    *connected_flag_clone.borrow_mut() = false;
                    let _ = state_sender.unbounded_send(false);
                }
                _ => {
                    console::warn("Unknown connection state");
                }
            }
        })
            as Box<dyn FnMut(web_sys::Event)>);

        self.connection.set_onconnectionstatechange(Some(
            onconnectionstatechange_callback.as_ref().unchecked_ref(),
        ));
        onconnectionstatechange_callback.forget();
        *self.connected.borrow_mut() = *connected_flag.borrow();

        let ice_connected = self.ice_connected.clone();
        let onicestatechange_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let state = connection_ref3.ice_connection_state();
            let is_connected = state == web_sys::RtcIceConnectionState::Connected
                || state == web_sys::RtcIceConnectionState::Completed;
            *ice_connected.borrow_mut() = is_connected;

            console::log(&format!(
                "ICE connection state changed to: {:?}, is_connected: {}",
                state, is_connected
            ));

            match state {
                web_sys::RtcIceConnectionState::New => {
                    console::log("ICE connection is new");
                }
                web_sys::RtcIceConnectionState::Checking => {
                    console::log("ICE connection is checking candidates...");
                }
                web_sys::RtcIceConnectionState::Connected => {
                    console::log("ICE connection established!");
                }
                web_sys::RtcIceConnectionState::Completed => {
                    console::log("ICE connection completed!");
                }
                web_sys::RtcIceConnectionState::Failed => {
                    console::log("ICE connection failed");
                }
                web_sys::RtcIceConnectionState::Disconnected => {
                    console::log("ICE connection disconnected");
                }
                web_sys::RtcIceConnectionState::Closed => {
                    console::log("ICE connection closed");
                }
                _ => {
                    console::warn("Unknown ICE connection state");
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.connection.set_oniceconnectionstatechange(Some(
            onicestatechange_callback.as_ref().unchecked_ref(),
        ));
        onicestatechange_callback.forget();

        let onicecandidate = {
            let peer_id = self.metadata.peer_id.clone();
            let remote_id_ref = Rc::new(RefCell::new(self.remote_peer_id.clone()));
            Closure::wrap(Box::new(move |ev: web_sys::RtcPeerConnectionIceEvent| {
                console::log(&format!(
                    "ICE candidate event triggered. Has candidate: {}",
                    ev.candidate().is_some()
                ));

                if let Some(candidate) = ev.candidate() {
                    let candidate_str = candidate.candidate();
                    console::log(&format!(
                        "ICE candidate details - sdp_m_line_index: {:?}, sdp_mid: {:?}, candidate: {}", 
                        candidate.sdp_m_line_index(),
                        candidate.sdp_mid(),
                        candidate_str
                    ));

                    if let Some(remote_id) = &*remote_id_ref.borrow() {
                        console::log(&format!(
                            "Sending ICE candidate to {}: {}",
                            remote_id, candidate_str
                        ));

                        let ice_msg = SignalingMessage::IceCandidate {
                            from: peer_id.clone(),
                            to: remote_id.clone(),
                            candidate: candidate_str,
                        };

                        with_signaling_manager(|manager| {
                            if let Some(signaling) = manager.get_client(&peer_id) {
                                let signaling_ref = signaling.borrow();
                                let websocket = signaling_ref.get_websocket();

                                if websocket.ready_state() != web_sys::WebSocket::OPEN {
                                    console::warn("WebSocket not ready, cannot send ICE candidate");
                                    return;
                                }

                                match serde_json::to_string(&ice_msg) {
                                    Ok(msg_str) => {
                                        console::log(&format!(
                                            "Sending ICE candidate message: {}",
                                            msg_str
                                        ));
                                        match websocket.send_with_str(&msg_str) {
                                            Ok(_) => {
                                                console::log("ICE candidate sent successfully")
                                            }
                                            Err(e) => console::error(&format!(
                                                "Failed to send ICE candidate: {:?}",
                                                e
                                            )),
                                        }
                                    }
                                    Err(e) => console::error(&format!(
                                        "Failed to serialize ICE candidate message: {:?}",
                                        e
                                    )),
                                }
                            } else {
                                console::error(
                                    "No signaling client found when trying to send ICE candidate",
                                );
                            }
                        });
                    } else {
                        console::warn("Generated ICE candidate but no remote peer ID set yet");
                    }
                } else {
                    console::log("ICE candidate gathering complete (null candidate)");
                }
            })
                as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>)
        };

        self.connection
            .set_onicecandidate(Some(onicecandidate.as_ref().unchecked_ref()));
        onicecandidate.forget();

        let channel_open = self.channel_open.clone();
        let message_sender = self.message_sender.clone();

        let ondatachannel_callback = {
            let channel_open_clone = channel_open.clone();
            let message_sender_clone = message_sender.clone();
            let data_channel_ref = Rc::new(RefCell::new(self.data_channel.clone()));

            Closure::wrap(Box::new(move |ev: web_sys::RtcDataChannelEvent| {
                console::log("Data channel received from remote peer");
                let channel = ev.channel();
                *data_channel_ref.borrow_mut() = Some(channel.clone());

                let channel_open_clone = channel_open_clone.clone();
                let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    console::log("Data channel opened (answerer)");
                    *channel_open_clone.borrow_mut() = true;
                }) as Box<dyn FnMut(web_sys::Event)>);
                channel.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                onopen.forget();

                let onclose = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    console::log("Data channel closed (answerer)");
                }) as Box<dyn FnMut(web_sys::Event)>);
                channel.set_onclose(Some(onclose.as_ref().unchecked_ref()));
                onclose.forget();

                let onerror = Closure::wrap(Box::new(move |e: web_sys::Event| {
                    console::error(&format!("Data channel error: {:?}", e));
                }) as Box<dyn FnMut(web_sys::Event)>);
                channel.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onerror.forget();

                let message_sender_clone = message_sender_clone.clone();
                let onmessage = Closure::wrap(Box::new(move |ev: MessageEvent| {
                    console::log("Message received on data channel");
                    if let Ok(data) = ev.data().dyn_into::<js_sys::ArrayBuffer>() {
                        let array = js_sys::Uint8Array::new(&data);
                        let mut vec = vec![0; array.length() as usize];
                        array.copy_to(&mut vec[..]);
                        console::log(&format!("Received message of {} bytes", vec.len()));

                        match serde_json::from_slice::<SyncMessage>(&vec) {
                            Ok(sync_msg) => {
                                console::log(&format!(
                                    "Received sync message for vault: {}, namespace: {}",
                                    sync_msg.vault_name, sync_msg.operation.namespace
                                ));

                                let vault_name = sync_msg.vault_name.clone();
                                let vec_clone = vec.clone();

                                wasm_bindgen_futures::spawn_local(async move {
                                    if let Err(e) =
                                        update_vault_from_sync(&vault_name, &vec_clone).await
                                    {
                                        console::error(&format!(
                                            "Failed to update vault {}: {:?}",
                                            vault_name, e
                                        ));
                                    } else {
                                        console::log(&format!(
                                            "Successfully updated vault {} from sync message",
                                            vault_name
                                        ));
                                    }
                                });
                            }
                            Err(e) => {
                                console::error(&format!("Failed to parse sync message: {}", e));
                            }
                        }

                        let _ = message_sender_clone.unbounded_send(vec);
                    }
                }) as Box<dyn FnMut(MessageEvent)>);
                channel.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                onmessage.forget();
            })
                as Box<dyn FnMut(web_sys::RtcDataChannelEvent)>)
        };

        self.connection
            .set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
        ondatachannel_callback.forget();

        if self.is_offerer {
            console::log("Creating data channel as offerer");

            let channel = self.connection.create_data_channel("data");
            console::log(&format!(
                "Data channel created with state: {:?}",
                channel.ready_state()
            ));
            self.data_channel = Some(channel.clone());

            let channel_open_clone = self.channel_open.clone();
            let connected_flag = self.connected.clone();
            let state_sender = self.connection_state_sender.clone();
            let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
                console::log("Data channel opened (offerer)");
                *channel_open_clone.borrow_mut() = true;
                *connected_flag.borrow_mut() = true;
                let _ = state_sender.unbounded_send(true);
                console::log("channel_open and connected flags set to true");
            }) as Box<dyn FnMut(web_sys::Event)>);
            channel.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();

            let connected_flag = self.connected.clone();
            let state_sender = self.connection_state_sender.clone();
            let onclose = Closure::wrap(Box::new(move |_: web_sys::Event| {
                console::log("Data channel closed (offerer)");
                *connected_flag.borrow_mut() = false;
                let _ = state_sender.unbounded_send(false);
            }) as Box<dyn FnMut(web_sys::Event)>);
            channel.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();

            let connected_flag = self.connected.clone();
            let state_sender = self.connection_state_sender.clone();
            let onerror = Closure::wrap(Box::new(move |e: web_sys::Event| {
                console::error(&format!("Data channel error: {:?}", e));
                *connected_flag.borrow_mut() = false;
                let _ = state_sender.unbounded_send(false);
            }) as Box<dyn FnMut(web_sys::Event)>);
            channel.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            let message_sender_clone = self.message_sender.clone();
            let onmessage = Closure::wrap(Box::new(move |ev: MessageEvent| {
                console::log("Message received on data channel");
                if let Ok(data) = ev.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(&data);
                    let mut vec = vec![0; array.length() as usize];
                    array.copy_to(&mut vec[..]);
                    console::log(&format!("Received message of {} bytes", vec.len()));

                    match serde_json::from_slice::<SyncMessage>(&vec) {
                        Ok(sync_msg) => {
                            console::log(&format!(
                                "Received sync message for vault: {}, namespace: {}",
                                sync_msg.vault_name, sync_msg.operation.namespace
                            ));
                        }
                        Err(e) => {
                            console::error(&format!("Failed to parse sync message: {}", e));
                        }
                    }

                    let _ = message_sender_clone.unbounded_send(vec);
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            channel.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();
        }

        console::log("WebRTC connection handlers setup complete");
        Ok(())
    }

    pub async fn create_offer(&self) -> Result<String, JsValue> {
        console::log("Creating WebRTC offer...");
        let offer = JsFuture::from(self.connection.create_offer()).await?;
        console::log("Setting local description...");

        let rtc_session_description_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        let sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("Failed to get SDP from offer"))?;
        rtc_session_description_init.set_sdp(&sdp);

        JsFuture::from(
            self.connection
                .set_local_description(&rtc_session_description_init),
        )
        .await?;
        console::log("Local description set successfully");

        Ok(sdp)
    }

    pub async fn create_answer(&self) -> Result<String, JsValue> {
        console::log("Creating WebRTC answer...");
        let answer = JsFuture::from(self.connection.create_answer()).await?;
        let sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("Failed to get answer SDP"))?;

        let desc_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        desc_init.set_sdp(&sdp);

        JsFuture::from(self.connection.set_local_description(&desc_init)).await?;
        Ok(sdp)
    }

    pub async fn handle_answer(&mut self, answer_sdp: &str) -> Result<(), JsValue> {
        console::log("Handle answer...");

        // Make sure we're the offerer
        if !self.is_offerer {
            console::error("Received answer but we're not the offerer!");
            return Err(JsValue::from_str(
                "Received answer but we're not the offerer",
            ));
        }

        let answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.set_sdp(answer_sdp);
        console::log(&format!(
            "Setting remote description (answer): {}",
            answer_sdp
        ));

        JsFuture::from(self.connection.set_remote_description(&answer_obj)).await?;
        console::log("Remote description (answer) set successfully");
        Ok(())
    }

    pub async fn handle_offer(&mut self, offer_sdp: &str) -> Result<String, JsValue> {
        console::log("Handle offer...");
        self.is_offerer = false;

        self.setup_connection().await?;

        let offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.set_sdp(offer_sdp);
        console::log("Setting remote description (offer)...");
        JsFuture::from(self.connection.set_remote_description(&offer_obj)).await?;
        console::log("Remote description set successfully");

        console::log("Creating answer...");
        let answer = JsFuture::from(self.connection.create_answer()).await?;
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
            .dyn_into::<JsString>()
            .map(String::from)
            .unwrap_or_default();
        console::log(&format!("Answer created: {}", answer_sdp));

        let answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.set_sdp(&answer_sdp);
        console::log("Setting local description (answer)...");
        JsFuture::from(self.connection.set_local_description(&answer_obj)).await?;
        console::log("Local description set successfully");

        if let Some(remote_id) = &self.remote_peer_id {
            console::log(&format!("Sending answer to remote peer {}", remote_id));
            if let Some(client) =
                with_signaling_manager(|mgr| mgr.get_client(&self.metadata.peer_id))
            {
                let client_ref = client.borrow();
                let websocket = client_ref.get_websocket();

                if websocket.ready_state() != web_sys::WebSocket::OPEN {
                    console::warn("WebSocket not ready, cannot send answer");
                    return Err(JsValue::from_str("WebSocket not ready, cannot send answer"));
                }

                let answer_msg = SignalingMessage::Answer {
                    from: self.metadata.peer_id.clone(),
                    to: remote_id.clone(),
                    sdp: answer_sdp.clone(),
                };

                if let Ok(msg_str) = serde_json::to_string(&answer_msg) {
                    console::log(&format!("Sending answer message: {}", msg_str));
                    match websocket.send_with_str(&msg_str) {
                        Ok(_) => console::log("Answer sent successfully"),
                        Err(e) => {
                            console::error(&format!("Failed to send answer: {:?}", e));
                            return Err(e);
                        }
                    }
                }
            }
        }

        Ok(answer_sdp)
    }

    pub async fn connect(
        &mut self,
        signaling_url: &str,
        target_peer_id: Option<&str>,
    ) -> Result<(), JsValue> {
        if *self.connected.borrow() {
            console::log("Already connected, skipping connection process");
            return Ok(());
        }

        console::log(&format!(
            "Starting WebRTC connection process. Target peer: {:?}",
            target_peer_id
        ));

        if let Some(target_id) = target_peer_id {
            console::log(&format!("Setting up as offerer for peer {}", target_id));
            self.remote_peer_id = Some(target_id.to_string());
            self.is_offerer = true;
        }

        console::log("Running connection setup...");
        self.setup_connection().await?;

        console::log(&format!(
            "Setting up signaling client for {} at {}",
            self.metadata.peer_id, signaling_url
        ));

        let signaling_receiver = with_signaling_manager(|mgr| {
            mgr.add_client(signaling_url, self.metadata.peer_id.clone())
        })?;

        let peer_id = self.metadata.peer_id.clone();
        let mut signaling_receiver = signaling_receiver;
        let peer = Rc::new(RefCell::new(self.clone()));

        wasm_bindgen_futures::spawn_local({
            let peer = peer.clone();
            async move {
                while let Some(msg) = signaling_receiver.next().await {
                    console::log(&format!(
                        "Received signaling message for {}: {:?}",
                        peer_id, msg
                    ));
                    let cloned_msg = msg.clone();
                    let peer_clone = Rc::clone(&peer);
                    let handle_message = async move {
                        match cloned_msg {
                            SignalingMessage::Offer { from, sdp, .. } => {
                                // Set remote peer ID
                                {
                                    let mut peer_ref = peer_clone.borrow_mut();
                                    peer_ref.remote_peer_id = Some(from.clone());
                                }

                                // Handle offer
                                let answer_sdp = {
                                    let mut peer_ref = peer_clone.borrow_mut();
                                    peer_ref.handle_offer(&sdp).await?
                                };

                                // Create and send answer
                                let peer_id = peer_clone.borrow().metadata.peer_id.clone();
                                let answer_msg = SignalingMessage::Answer {
                                    from: peer_id.clone(),
                                    to: from.clone(),
                                    sdp: answer_sdp,
                                };

                                if let Ok(msg_str) = serde_json::to_string(&answer_msg) {
                                    if let Some(client) =
                                        with_signaling_manager(|mgr| mgr.get_client(&peer_id))
                                    {
                                        let client_ref = client.borrow();
                                        let websocket = client_ref.get_websocket();

                                        if websocket.ready_state() != web_sys::WebSocket::OPEN {
                                            console::warn(
                                                "WebSocket not ready, cannot send answer",
                                            );
                                            return Err(JsValue::from_str(
                                                "WebSocket not ready, cannot send answer",
                                            ));
                                        }

                                        websocket.send_with_str(&msg_str)?;
                                    }
                                }
                                Ok(())
                            }
                            SignalingMessage::Answer { from, sdp, .. } => {
                                // Set remote peer ID
                                {
                                    let mut peer_ref = peer_clone.borrow_mut();
                                    peer_ref.remote_peer_id = Some(from.clone());
                                }

                                // Handle answer
                                let mut peer_ref = peer_clone.borrow_mut();
                                peer_ref.handle_answer(&sdp).await?;
                                Ok(())
                            }
                            SignalingMessage::IceCandidate {
                                candidate, ..
                            } => {
                                let peer_ref = peer_clone.borrow_mut();
                                peer_ref.handle_ice_candidate(&candidate).await?;
                                Ok(())
                            }
                            _ => Ok(()),
                        }
                    };

                    if let Err(e) = handle_message.await {
                        console::error(&format!("Error handling signaling message: {:?}", e));
                    }
                }
            }
        });

        console::log("Waiting for WebSocket connection...");
        let ws_ready = js_sys::Promise::new(&mut |resolve, reject| {
            let peer_id = self.metadata.peer_id.clone();
            let reject_clone = reject.clone();

            if let Some(client) = with_signaling_manager(|mgr| mgr.get_client(&peer_id)) {
                let client_ref = client.borrow();
                if client_ref.get_websocket().ready_state() == web_sys::WebSocket::OPEN {
                    console::log("WebSocket already connected");
                    resolve.call0(&JsValue::NULL).unwrap_or_default();
                    return;
                }
            }

            let onopen = {
                let peer_id = peer_id.clone();
                let reject = reject_clone.clone();
                Closure::wrap(Box::new(move || {
                    console::log("WebSocket connection opened");

                    if let Some(client) = with_signaling_manager(|mgr| mgr.get_client(&peer_id)) {
                        let join_msg = SignalingMessage::Join {
                            peer_id: peer_id.clone(),
                        };
                        if let Ok(msg_str) = serde_json::to_string(&join_msg) {
                            console::log(&format!("Sending join message: {}", msg_str));
                            match client.borrow().get_websocket().send_with_str(&msg_str) {
                                Ok(_) => console::log("Join message sent successfully"),
                                Err(e) => {
                                    console::error(&format!(
                                        "Failed to send join message: {:?}",
                                        e
                                    ));
                                    reject.call1(&JsValue::NULL, &e).unwrap_or_default();
                                    return;
                                }
                            }
                        }
                    }
                    resolve.call0(&JsValue::NULL).unwrap_or_default();
                }) as Box<dyn FnMut()>)
            };

            let onerror = {
                let reject = reject_clone;
                Closure::wrap(Box::new(move |e: ErrorEvent| {
                    console::error(&format!("WebSocket error: {:?}", e));
                    reject.call1(&JsValue::NULL, &e.into()).unwrap_or_default();
                }) as Box<dyn FnMut(ErrorEvent)>)
            };

            if let Some(client) = with_signaling_manager(|mgr| mgr.get_client(&peer_id)) {
                let client_ref = client.borrow();
                let ws = client_ref.get_websocket();
                ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onopen.forget();
                onerror.forget();
            } else {
                reject
                    .call1(
                        &JsValue::NULL,
                        &JsValue::from_str("No signaling client found"),
                    )
                    .unwrap_or_default();
            }
        });

        console::log("Awaiting WebSocket ready promise...");
        JsFuture::from(ws_ready).await?;
        console::log("WebSocket connection established");

        if self.is_offerer {
            if let Some(target_id) = &self.remote_peer_id {
                console::log("Creating offer as offerer...");
                let offer = self.create_offer().await?;

                let offer_msg = SignalingMessage::Offer {
                    from: self.metadata.peer_id.clone(),
                    to: target_id.clone(),
                    sdp: offer,
                };

                if let Ok(msg_str) = serde_json::to_string(&offer_msg) {
                    console::log(&format!(
                        "Sending offer from {} to {}: {}",
                        self.metadata.peer_id, target_id, msg_str
                    ));
                    if let Some(client) =
                        with_signaling_manager(|mgr| mgr.get_client(&self.metadata.peer_id))
                    {
                        let client_ref = client.borrow();
                        let ws = client_ref.get_websocket();
                        console::log(&format!(
                            "WebSocket state before sending offer: {:?}",
                            ws.ready_state()
                        ));
                        if let Err(e) = ws.send_with_str(&msg_str) {
                            console::error(&format!("Failed to send offer: {:?}", e));
                            return Err(e);
                        }
                        console::log("Offer sent successfully");
                    }
                }
            }
        }

        console::log("Connection setup complete. Waiting for peer connection to establish...");
        Ok(())
    }

    pub fn send_message(&self, data: Vec<u8>) -> Result<(), JsValue> {
        if let Some(channel) = &self.data_channel {
            let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
            array.copy_from(&data);
            channel.send_with_array_buffer(&array.buffer())?;
        }
        Ok(())
    }

    pub fn add_permission(&mut self, namespace: String, access_level: AccessLevel) {
        self.metadata.permissions.insert(namespace, access_level);
    }

    pub fn has_permission(&self, namespace: &str, required_level: AccessLevel) -> bool {
        self.metadata
            .permissions
            .get(namespace)
            .map_or(false, |level| {
                matches!(
                    (required_level, level),
                    (AccessLevel::Viewer, _)
                        | (
                            AccessLevel::Contributor,
                            AccessLevel::Contributor | AccessLevel::Administrator
                        )
                        | (AccessLevel::Administrator, AccessLevel::Administrator)
                )
            })
    }

    pub async fn handle_connection_state_update(&mut self) {
        let (state_sender, mut state_receiver) = mpsc::unbounded();
        self.connection_state_sender = state_sender;

        let connected = self.connected.clone();

        wasm_bindgen_futures::spawn_local({
            async move {
                while let Some(is_connected) = state_receiver.next().await {
                    *connected.borrow_mut() = is_connected;
                    console::log(&format!("Updated connection state: {}", is_connected));
                }
            }
        });
    }

    pub async fn handle_ice_candidate(&self, candidate_str: &str) -> Result<(), JsValue> {
        console::log(&format!(
            "Handling incoming ICE candidate: {}",
            candidate_str
        ));

        let candidate_init = RtcIceCandidateInit::new(candidate_str);
        candidate_init.set_sdp_mid(Some("0"));
        candidate_init.set_sdp_m_line_index(Some(0));

        match RtcIceCandidate::new(&candidate_init) {
            Ok(candidate) => {
                console::log(&format!(
                    "Created ICE candidate object: sdp_mid={:?}, sdp_m_line_index={:?}",
                    candidate.sdp_mid(),
                    candidate.sdp_m_line_index()
                ));

                match JsFuture::from(
                    self.connection
                        .add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate)),
                )
                .await
                {
                    Ok(_) => {
                        console::log("Successfully added ICE candidate");
                        Ok(())
                    }
                    Err(e) => {
                        console::error(&format!("Failed to add ICE candidate: {:?}", e));
                        Err(e)
                    }
                }
            }
            Err(e) => {
                console::error(&format!("Failed to create ICE candidate: {:?}", e));
                Err(e)
            }
        }
    }
}
