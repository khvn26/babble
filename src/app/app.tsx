import { useMemo } from "react";
import { useAppStore } from "../stores/app-store";

function Pill({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded-full bg-white/10 px-2 py-1 text-xs text-white/80">
      {children}
    </span>
  );
}

export default function App() {
  const {
    channels,
    connect,
    connState,
    currentChannelId,
    currentServer,
    disconnect,
    me,
    selectChannel,
    toggleDeafen,
    toggleMute,
    users,
  } = useAppStore();

  const selectedChannelId = currentChannelId ?? channels[0]?.id ?? 0;

  const usersHere = useMemo(
    () => users.filter((u) => u.channelId === selectedChannelId),
    [users, selectedChannelId]
  );

  return (
    <div className="h-full bg-zinc-950 text-zinc-100">
      <div className="grid h-full grid-cols-[72px_260px_1fr]">
        {/* Servers rail */}
        <aside className="border-r border-white/10 p-3">
          <div className="flex flex-col gap-3">
            <button className="h-12 w-12 rounded-2xl bg-indigo-500/90 hover:rounded-xl transition-all" />
            <button className="h-12 w-12 rounded-2xl bg-white/10 hover:bg-white/15 hover:rounded-xl transition-all" />
            <button className="h-12 w-12 rounded-2xl bg-white/10 hover:bg-white/15 hover:rounded-xl transition-all" />
          </div>
        </aside>

        {/* Channels */}
        <section className="border-r border-white/10 flex flex-col">
          <div className="p-3 border-b border-white/10 flex items-center justify-between">
            <div className="font-semibold">{currentServer ?? "Server Name"}</div>
            <Pill>{connState}</Pill>
          </div>

          <div className="p-3">
            <input
              className="w-full rounded-lg bg-white/5 px-3 py-2 text-sm outline-none ring-1 ring-white/10 focus:ring-2 focus:ring-indigo-500/60"
              placeholder="Search channels (Ctrl+K later)"
            />
          </div>

          <div className="px-2 pb-2 overflow-auto">
            {channels.map((c) => {
              const active = c.id === selectedChannelId;
              return (
                <button
                  key={c.id}
                  onClick={() => selectChannel(c.id)}
                  className={[
                    "w-full rounded-lg px-3 py-2 text-left text-sm",
                    active ? "bg-white/10" : "hover:bg-white/5",
                  ].join(" ")}
                >
                  # {c.name}
                </button>
              );
            })}
          </div>

          {/* Bottom-left controls */}
          <div className="mt-auto border-t border-white/10 p-3 flex items-center justify-between">
            <div className="flex flex-col">
              <span className="text-sm font-semibold leading-tight">You</span>
              <span className="text-xs text-white/60">Voice connected</span>
            </div>

            <div className="flex gap-2">
              <button
                onClick={toggleMute}
                className={[
                  "rounded-lg px-3 py-2 text-xs ring-1 ring-white/10 hover:bg-white/10",
                  me.muted ? "bg-red-500/30" : "bg-white/5",
                ].join(" ")}
              >
                {me.muted ? "Muted" : "Mute"}
              </button>
              <button
                onClick={toggleDeafen}
                className={[
                  "rounded-lg px-3 py-2 text-xs ring-1 ring-white/10 hover:bg-white/10",
                  me.deafened ? "bg-yellow-500/25" : "bg-white/5",
                ].join(" ")}
              >
                {me.deafened ? "Deaf" : "Deafen"}
              </button>
              <button
                onClick={() =>
                  connState === "connected" ? disconnect() : connect("localhost")
                }
                className="rounded-lg bg-white/5 px-3 py-2 text-xs ring-1 ring-white/10 hover:bg-white/10"
              >
                {connState === "connected" ? "Disconnect" : "Connect"}
              </button>
            </div>
          </div>
        </section>

        {/* Main voice room */}
        <main className="p-6">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-lg font-semibold">
                # {channels.find((c) => c.id === selectedChannelId)?.name}
              </div>
              <div className="text-sm text-white/60">People</div>
            </div>
            <div className="flex gap-2">
              <Pill>VAD</Pill>
              <Pill>PTT (later)</Pill>
            </div>
          </div>

          <div className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {usersHere.map((u) => (
              <div
                key={u.id}
                className={[
                  "rounded-2xl border border-white/10 bg-white/5 p-4",
                  u.talking ? "ring-2 ring-indigo-500/60" : "",
                ].join(" ")}
              >
                <div className="flex items-center justify-between">
                  <div className="font-semibold">{u.name}</div>
                  <div className="text-xs text-white/60">
                    {u.talking ? "Speaking" : "Idle"}
                  </div>
                </div>
                <div className="mt-3 flex gap-2 text-xs text-white/70">
                  {u.muted && <Pill>Muted</Pill>}
                  {u.deafened && <Pill>Deaf</Pill>}
                </div>
              </div>
            ))}
          </div>
        </main>
      </div>
    </div>
  );
}
