import { create } from "zustand";
import { createMockTransport } from "../transports/mock-transport";
import type { TransportState } from "../transports/types";

type AppState = TransportState & {
  connect: (server: string) => Promise<void>;
  disconnect: () => Promise<void>;
  selectChannel: (channelId: number) => Promise<void>;
  toggleMute: () => Promise<void>;
  toggleDeafen: () => Promise<void>;
};

const transport = createMockTransport();

export const useAppStore = create<AppState>((set, get) => {
  const initial = transport.getState();
  transport.subscribe((state) => set(state));

  return {
    ...initial,
    connect: (server) => transport.connect({ server }),
    disconnect: () => transport.disconnect(),
    selectChannel: (channelId) => transport.joinChannel(channelId),
    toggleMute: () => transport.setMute(!get().me.muted),
    toggleDeafen: () => transport.setDeafen(!get().me.deafened),
  };
});
