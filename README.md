# babble

An attempt to build a user-friendly Mumble client.

Target platform: Windows 10 (officially supported).

The rough plan is:

1. Familiar (Discord-like) UI/UX
1. Auto cert gen
1. Auto-connect to last server
1. Direct Mumble transport (Windows)
1. WebSockets + WebRTC transport via mumble-web-proxy (Web)

Later/maybe:

1. In-game overlay

## Native transport scope (v1)

Minimum feature set:
- Connect
- Channel list
- User list
- Mute/deafen
- Text chat
- Voice

Auth:
- Username/password
- Client certificate (if present)

Servers:
- Any Mumble server

Audio:
- VAD (no push-to-talk in v1)
- Device selection (prefer WASAPI; ASIO later)

## Transport API (app-facing)

Methods:
- connect({ server, port, username, password?, cert? })
- disconnect()
- setSelfMute(muted)
- setSelfDeafen(deafened)
- sendText({ channelId?, userId?, message })
- joinChannel(channelId)
- setAudioInputDevice(deviceId)
- setAudioOutputDevice(deviceId)
- setVAD(enabled, threshold?)

State:
- connState, currentServer, currentChannelId
- channels[], users[], me
- audio: { inputDevices[], outputDevices[], inputDeviceId?, outputDeviceId?, vadEnabled, vadLevel? }

## Mumble protocol mapping (high-level)

- TLS handshake + Version + Authenticate
- ServerSync + ChannelState + UserState
- TextMessage (channel/user)
- Voice over UDP (Opus) + CryptSetup/CodecVersion

## Testing (native)

- Run coverage tests: `npm run tauri:test`
- Coverage uses the Cargo feature `coverage` to avoid running the Tauri runtime in unit tests.
