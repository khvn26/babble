import { create } from "zustand";

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

type AppState = {
    connState: ConnState;
    currentServer?: string;
    currentChannelId?: number;

    me: { muted: boolean; deafened: boolean };
    channels: Channel[];
    users: User[];

    setConnState: (s: ConnState) => void;
    setCurrent: (server: string, channelId?: number) => void;
    toggleMute: () => void;
    toggleDeafen: () => void;
};

export const useAppStore = create<AppState>((set) => ({
    connState: "disconnected",
    me: { muted: false, deafened: false },
    channels: [
        { id: 1, name: "Lobby" },
        { id: 2, name: "Gaming" },
        { id: 3, name: "AFK" },
    ],
    users: [
        { id: 1, name: "You", channelId: 1 },
        { id: 2, name: "Alex", channelId: 1, talking: true },
        { id: 3, name: "Sam", channelId: 2 },
    ],

    setConnState: (connState) => set({ connState }),
    setCurrent: (currentServer, currentChannelId) =>
        set({ currentServer, currentChannelId }),
    toggleMute: () => set((s) => ({ me: { ...s.me, muted: !s.me.muted } })),
    toggleDeafen: () =>
        set((s) => ({ me: { ...s.me, deafened: !s.me.deafened } })),
}));
