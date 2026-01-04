export type ConnState = "disconnected" | "connecting" | "connected" | "error";

export type Channel = { id: number; name: string; parentId?: number };
export type User = {
  id: number;
  name: string;
  channelId: number;
  talking?: boolean;
  muted?: boolean;
  deafened?: boolean;
};

export type MeState = { muted: boolean; deafened: boolean };

export type TransportState = {
  connState: ConnState;
  currentServer?: string;
  currentChannelId?: number;
  me: MeState;
  channels: Channel[];
  users: User[];
};

export type TransportListener = (state: TransportState) => void;

export type ConnectParams = {
  server: string;
  username?: string;
};

export interface Transport {
  getState(): TransportState;
  subscribe(listener: TransportListener): () => void;
  connect(params: ConnectParams): Promise<void>;
  disconnect(): Promise<void>;
  joinChannel(channelId: number): Promise<void>;
  setMute(muted: boolean): Promise<void>;
  setDeafen(deafened: boolean): Promise<void>;
}
