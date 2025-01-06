use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JoinPayload {
    pub peer_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeavePayload {
    pub peer_id: String,
}
