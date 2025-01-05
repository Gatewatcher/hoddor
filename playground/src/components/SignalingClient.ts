export interface SignalingConfig {
    serverUrl: string;
    authToken?: string;
}

export type SignalingMessageType = 
    | { type: 'join', peer_id: string }
    | { type: 'offer', from: string, to: string, sdp: string }
    | { type: 'answer', from: string, to: string, sdp: string }
    | { type: 'ice_candidate', from: string, to: string, candidate: string }
    | { type: 'leave', peer_id: string };

export class SignalingClient {
    private ws: WebSocket | null = null;
    private messageHandlers: ((msg: SignalingMessageType) => void)[] = [];
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000;
    private authToken: string | undefined;

    constructor(private config: SignalingConfig) {
        this.authToken = config.authToken;
    }

    async connect(): Promise<void> {
        if (!this.authToken) {
            this.authToken = await this.fetchAuthToken();
        }

        return new Promise((resolve, reject) => {
            try {
                this.ws = new WebSocket(this.config.serverUrl);
                
                this.ws.onopen = () => {
                    this.reconnectAttempts = 0;
                    resolve();
                };

                this.ws.onmessage = (event) => {
                    try {
                        const message = JSON.parse(event.data) as SignalingMessageType;
                        this.messageHandlers.forEach(handler => handler(message));
                    } catch (error) {
                        console.error('Failed to parse signaling message:', error);
                    }
                };

                this.ws.onclose = () => {
                    this.handleDisconnect();
                };

                this.ws.onerror = (error) => {
                    console.error('WebSocket error:', error);
                    reject(error);
                };

                // Add token to WebSocket headers
                if (this.ws.readyState === WebSocket.CONNECTING) {
                    this.ws.addEventListener('open', () => {
                        if (this.ws && this.authToken) {
                            const headers = {
                                'Authorization': `Bearer ${this.authToken}`
                            };
                            // Send auth headers in a message since WebSocket doesn't support custom headers
                            this.ws.send(JSON.stringify({ type: 'auth', headers }));
                        }
                    });
                }
            } catch (error) {
                reject(error);
            }
        });
    }

    private async fetchAuthToken(): Promise<string> {
        try {
            const response = await fetch(`${new URL(this.config.serverUrl).origin}/token`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                }
            });

            if (!response.ok) {
                throw new Error('Failed to fetch auth token');
            }

            const data = await response.json();
            return data.token;
        } catch (error) {
            console.error('Error fetching auth token:', error);
            throw error;
        }
    }

    private async handleDisconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
            console.log(`Attempting to reconnect in ${delay}ms...`);
            
            setTimeout(async () => {
                try {
                    await this.connect();
                } catch (error) {
                    console.error('Reconnection failed:', error);
                }
            }, delay);
        } else {
            console.error('Max reconnection attempts reached');
        }
    }

    send(message: SignalingMessageType): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        } else {
            console.error('WebSocket is not connected');
        }
    }

    onMessage(handler: (msg: SignalingMessageType) => void): void {
        this.messageHandlers.push(handler);
    }

    close(): void {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}
