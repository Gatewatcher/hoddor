import { Vault } from '../wasm';
import { SignalingClient, SignalingMessageType } from './SignalingClient';

export interface VaultSyncConfig {
    serverUrl: string;
    stun_servers?: string[];
    reconnectAttempts?: number;
    reconnectDelay?: number;
}

export class VaultSync {
    private vault: Vault;
    private signaling: SignalingClient | null = null;
    private peer_id: string | null = null;
    private connected_peers: Set<string> = new Set();
    private config: VaultSyncConfig;

    constructor(vault: Vault, config: VaultSyncConfig) {
        this.vault = vault;
        this.config = {
            stun_servers: ['stun:stun.l.google.com:19302'],
            reconnectAttempts: 5,
            reconnectDelay: 1000,
            ...config
        };
    }

    async enableSync(): Promise<string> {
        try {
            // Initialize signaling client
            this.signaling = new SignalingClient({
                serverUrl: this.config.serverUrl
            });

            // Connect to signaling server
            await this.signaling.connect();

            // Enable vault sync with WebRTC
            this.peer_id = await this.vault.enable_sync(
                this.config.serverUrl,
                this.config.stun_servers || []
            );

            // Join the signaling room
            this.signaling.send({
                type: 'join',
                peer_id: this.peer_id
            });

            // Set up message handling
            this.signaling.onMessage(this.handleSignalingMessage.bind(this));

            return this.peer_id;
        } catch (error) {
            console.error('Failed to enable sync:', error);
            throw error;
        }
    }

    private async handleSignalingMessage(message: SignalingMessageType) {
        try {
            switch (message.type) {
                case 'offer':
                    if (message.to === this.peer_id) {
                        await this.vault.handle_offer(message.from, message.sdp);
                    }
                    break;
                case 'answer':
                    if (message.to === this.peer_id) {
                        await this.vault.handle_answer(message.from, message.sdp);
                    }
                    break;
                case 'ice_candidate':
                    if (message.to === this.peer_id) {
                        await this.vault.handle_ice_candidate(message.from, message.candidate);
                    }
                    break;
            }
        } catch (error) {
            console.error('Error handling signaling message:', error);
        }
    }

    async connectToPeer(peer_id: string): Promise<void> {
        if (!this.peer_id || !this.signaling) {
            throw new Error('Sync not enabled. Call enableSync() first.');
        }

        try {
            await this.vault.connect_to_peer(peer_id);
            this.connected_peers.add(peer_id);
        } catch (error) {
            console.error('Failed to connect to peer:', error);
            throw error;
        }
    }

    async addPeerPermission(
        peer_id: string,
        namespace: string,
        access_level: 'viewer' | 'contributor' | 'administrator'
    ): Promise<void> {
        if (!this.connected_peers.has(peer_id)) {
            throw new Error('Peer not connected');
        }

        try {
            await this.vault.add_peer(peer_id, namespace, access_level);
        } catch (error) {
            console.error('Failed to add peer permission:', error);
            throw error;
        }
    }

    getConnectedPeers(): string[] {
        return Array.from(this.connected_peers);
    }

    getPeerId(): string {
        if (!this.peer_id) {
            throw new Error('Sync not enabled');
        }
        return this.peer_id;
    }

    disconnect(): void {
        if (this.signaling && this.peer_id) {
            this.signaling.send({
                type: 'leave',
                peer_id: this.peer_id
            });
            this.signaling.close();
            this.signaling = null;
        }
        this.connected_peers.clear();
        this.peer_id = null;
    }
}
