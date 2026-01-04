import type {
  ConnectParams,
  Transport,
  TransportListener,
  TransportState,
  User,
} from "./types";

const defaultChannels = [
  { id: 1, name: "Lobby" },
  { id: 2, name: "Gaming" },
  { id: 3, name: "AFK" },
];

const defaultUsers: User[] = [
  { id: 1, name: "You", channelId: 1 },
  { id: 2, name: "Alex", channelId: 1, talking: true },
  { id: 3, name: "Sam", channelId: 2 },
];

class MockTransport implements Transport {
  private state: TransportState = {
    connState: "disconnected",
    currentServer: undefined,
    currentChannelId: 1,
    me: { muted: false, deafened: false },
    channels: defaultChannels,
    users: defaultUsers,
  };
  private listeners = new Set<TransportListener>();
  private connectTimer: ReturnType<typeof setTimeout> | undefined;

  getState(): TransportState {
    return this.snapshot();
  }

  subscribe(listener: TransportListener): () => void {
    this.listeners.add(listener);
    listener(this.snapshot());
    return () => {
      this.listeners.delete(listener);
    };
  }

  async connect(params: ConnectParams): Promise<void> {
    if (this.state.connState === "connected") return;
    this.update({ connState: "connecting", currentServer: params.server });
    if (this.connectTimer) clearTimeout(this.connectTimer);
    this.connectTimer = setTimeout(() => {
      this.update({ connState: "connected" });
    }, 400);
  }

  async disconnect(): Promise<void> {
    if (this.connectTimer) clearTimeout(this.connectTimer);
    this.update({ connState: "disconnected" });
  }

  async joinChannel(channelId: number): Promise<void> {
    const users = this.state.users.map((user) =>
      user.id === 1 ? { ...user, channelId } : user
    );
    this.update({ currentChannelId: channelId, users });
  }

  async setMute(muted: boolean): Promise<void> {
    this.updateMe({ muted });
  }

  async setDeafen(deafened: boolean): Promise<void> {
    this.updateMe({ deafened });
  }

  private updateMe(partial: { muted?: boolean; deafened?: boolean }) {
    const me = { ...this.state.me, ...partial };
    const users = this.state.users.map((user) =>
      user.id === 1 ? { ...user, muted: me.muted, deafened: me.deafened } : user
    );
    this.update({ me, users });
  }

  private update(partial: Partial<TransportState>) {
    this.state = { ...this.state, ...partial };
    const snapshot = this.snapshot();
    this.listeners.forEach((listener) => listener(snapshot));
  }

  private snapshot(): TransportState {
    return {
      ...this.state,
      me: { ...this.state.me },
      channels: this.state.channels.map((channel) => ({ ...channel })),
      users: this.state.users.map((user) => ({ ...user })),
    };
  }
}

export function createMockTransport(): Transport {
  return new MockTransport();
}
