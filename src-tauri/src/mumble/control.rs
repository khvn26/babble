use crate::transport::errors::TransportError;
use bytes::BytesMut;
use mumble_protocol_2x::control::{msgs, ControlPacket};
use mumble_protocol_2x::voice::{Clientbound, Serverbound};
#[cfg(not(feature = "coverage"))]
use openssl::ssl::{SslConnector, SslMethod};
#[cfg(not(feature = "coverage"))]
use std::net::TcpStream;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlMessage {
    ServerSync {
        session: u32,
    },
    ChannelState {
        id: u32,
        name: String,
        parent_id: Option<u32>,
    },
    UserState {
        id: u32,
        name: String,
        channel_id: u32,
        muted: bool,
        deafened: bool,
        talking: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserStateCommand {
    pub session_id: u32,
    pub channel_id: u32,
    pub muted: Option<bool>,
    pub deafened: Option<bool>,
}

pub struct ControlHandshake {
    pub messages: Vec<ControlMessage>,
    pub session: Option<Box<dyn ControlSession>>,
}

impl std::fmt::Debug for ControlHandshake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlHandshake")
            .field("messages", &self.messages)
            .field("session_present", &self.session.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
}

pub trait ControlConnector {
    fn handshake(&mut self, request: HandshakeRequest) -> Result<ControlHandshake, TransportError>;
}

pub trait ControlSession {
    fn send_user_state(&mut self, command: UserStateCommand) -> Result<(), TransportError>;
}

pub trait ControlTransport {
    fn send(&mut self, packet: ControlPacket<Serverbound>) -> Result<(), TransportError>;
    fn recv(&mut self) -> Result<Option<ControlPacket<Clientbound>>, TransportError>;
}

#[derive(Debug, Default)]
pub struct NoopControlConnector;

impl ControlConnector for NoopControlConnector {
    fn handshake(
        &mut self,
        _request: HandshakeRequest,
    ) -> Result<ControlHandshake, TransportError> {
        Ok(ControlHandshake {
            messages: Vec::new(),
            session: None,
        })
    }
}

pub struct MumbleProtocolControlConnector<T: ControlTransport> {
    transport: Option<T>,
}

pub struct SocketControlConnector<F> {
    connect: F,
}

pub struct BlockingControlTransport<S> {
    stream: S,
    codec: mumble_protocol_2x::control::ClientControlCodec,
    read_buf: BytesMut,
}

impl<S> BlockingControlTransport<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            codec: mumble_protocol_2x::control::ClientControlCodec::new(),
            read_buf: BytesMut::with_capacity(4096),
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

#[cfg(not(feature = "coverage"))]
pub fn tls_connect(
    request: &HandshakeRequest,
) -> Result<openssl::ssl::SslStream<TcpStream>, TransportError> {
    let address = format!("{}:{}", request.server, request.port);
    let tcp = TcpStream::connect(address)?;
    let builder = SslConnector::builder(SslMethod::tls())
        .map_err(|err| TransportError::Io(format!("tls connector init failed: {err}")))?;
    let connector = builder.build();
    connector
        .connect(&request.server, tcp)
        .map_err(|err| TransportError::Io(format!("tls handshake failed: {err}")))
}

impl<F> SocketControlConnector<F> {
    pub fn new(connect: F) -> Self {
        Self { connect }
    }
}

impl<S: std::io::Read + std::io::Write> ControlTransport for BlockingControlTransport<S> {
    fn send(&mut self, packet: ControlPacket<Serverbound>) -> Result<(), TransportError> {
        let mut out = BytesMut::with_capacity(512);
        self.codec.encode(packet, &mut out)?;
        self.stream.write_all(&out)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<ControlPacket<Clientbound>>, TransportError> {
        loop {
            if let Some(packet) = self.codec.decode(&mut self.read_buf)? {
                return Ok(Some(packet));
            }

            let mut buffer = [0u8; 4096];
            let bytes_read = self.stream.read(&mut buffer)?;
            if bytes_read == 0 {
                return Ok(None);
            }
            self.read_buf.extend_from_slice(&buffer[..bytes_read]);
        }
    }
}

impl<T: ControlTransport> MumbleProtocolControlConnector<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Some(transport),
        }
    }

    fn map_control_packet(packet: ControlPacket<Clientbound>) -> Option<ControlMessage> {
        match packet {
            ControlPacket::ServerSync(msg) => {
                let session = msg.session?;
                Some(ControlMessage::ServerSync { session })
            }
            ControlPacket::ChannelState(msg) => {
                let id = msg.channel_id?;
                let name = msg.name.clone()?;
                Some(ControlMessage::ChannelState {
                    id,
                    name,
                    parent_id: msg.parent,
                })
            }
            ControlPacket::UserState(msg) => {
                let id = msg.session?;
                let name = msg.name.clone()?;
                let channel_id = msg.channel_id?;
                let muted = msg.self_mute.unwrap_or(false);
                let deafened = msg.self_deaf.unwrap_or(false);
                Some(ControlMessage::UserState {
                    id,
                    name,
                    channel_id,
                    muted,
                    deafened,
                    talking: false,
                })
            }
            _ => None,
        }
    }
}

impl<T: ControlTransport + 'static> ControlConnector for MumbleProtocolControlConnector<T> {
    fn handshake(&mut self, request: HandshakeRequest) -> Result<ControlHandshake, TransportError> {
        let mut transport = self.transport.take().ok_or_else(|| {
            TransportError::Protocol("control transport already consumed".to_string())
        })?;
        let mut auth = msgs::Authenticate::new();
        auth.username = Some(request.username);
        auth.password = request.password;

        let packet = ControlPacket::Authenticate(Box::new(auth));
        transport.send(packet)?;

        let mut messages = Vec::new();
        while let Some(packet) = transport.recv()? {
            if let Some(message) = Self::map_control_packet(packet) {
                messages.push(message);
            }
        }

        Ok(ControlHandshake {
            messages,
            session: Some(Box::new(MumbleProtocolControlSession { transport })),
        })
    }
}

impl<F, S> ControlConnector for SocketControlConnector<F>
where
    F: FnMut(&HandshakeRequest) -> Result<S, TransportError>,
    S: std::io::Read + std::io::Write + 'static,
{
    fn handshake(&mut self, request: HandshakeRequest) -> Result<ControlHandshake, TransportError> {
        let stream = (self.connect)(&request)?;
        let transport = BlockingControlTransport::new(stream);
        let mut connector = MumbleProtocolControlConnector::new(transport);
        connector.handshake(request)
    }
}

pub struct MumbleProtocolControlSession<T: ControlTransport> {
    transport: T,
}

impl<T: ControlTransport + 'static> ControlSession for MumbleProtocolControlSession<T> {
    fn send_user_state(&mut self, command: UserStateCommand) -> Result<(), TransportError> {
        let mut message = msgs::UserState::new();
        message.session = Some(command.session_id);
        message.channel_id = Some(command.channel_id);
        message.self_mute = command.muted;
        message.self_deaf = command.deafened;
        self.transport
            .send(ControlPacket::UserState(Box::new(message)))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlockingControlTransport, ControlConnector, ControlMessage, ControlTransport,
        HandshakeRequest, MumbleProtocolControlConnector, SocketControlConnector,
    };
    use crate::transport::errors::TransportError;
    use mumble_protocol_2x::control::{msgs, ControlPacket};
    use mumble_protocol_2x::voice::{Clientbound, Serverbound};
    use std::cell::RefCell;
    use std::io::{Cursor, Read, Write};
    use std::rc::Rc;
    use tokio_util::codec::{Decoder, Encoder};

    struct TestTransport {
        sent: Rc<RefCell<Vec<ControlPacket<Serverbound>>>>,
        recv_queue: Vec<ControlPacket<Clientbound>>,
        send_error: bool,
        recv_error: bool,
    }

    impl Default for TestTransport {
        fn default() -> Self {
            Self {
                sent: Rc::new(RefCell::new(Vec::new())),
                recv_queue: Vec::new(),
                send_error: false,
                recv_error: false,
            }
        }
    }

    impl ControlTransport for TestTransport {
        fn send(&mut self, packet: ControlPacket<Serverbound>) -> Result<(), TransportError> {
            if self.send_error {
                return Err(TransportError::Io("send failed".to_string()));
            }
            self.sent.borrow_mut().push(packet);
            Ok(())
        }

        fn recv(&mut self) -> Result<Option<ControlPacket<Clientbound>>, TransportError> {
            if self.recv_error {
                return Err(TransportError::Io("recv failed".to_string()));
            }
            if self.recv_queue.is_empty() {
                Ok(None)
            } else {
                Ok(Some(self.recv_queue.remove(0)))
            }
        }
    }

    #[derive(Default)]
    struct MemoryStream {
        read: Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    impl MemoryStream {
        fn with_read_data(data: Vec<u8>) -> Self {
            Self {
                read: Cursor::new(data),
                written: Vec::new(),
            }
        }
    }

    impl Read for MemoryStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.read.read(buf)
        }
    }

    impl Write for MemoryStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// In-memory stream flush is a no-op.
    #[test]
    fn memory_stream_flush_is_noop() {
        // Arrange
        let mut stream = MemoryStream::default();
        // Act
        let result = stream.flush();
        // Assert
        result.expect("flush failed");
    }

    /// Handshake sends an authenticate control packet with credentials.
    #[test]
    fn handshake_sends_authenticate() {
        // Arrange
        let sent = Rc::new(RefCell::new(Vec::new()));
        let transport = TestTransport {
            sent: Rc::clone(&sent),
            ..Default::default()
        };
        let mut connector = MumbleProtocolControlConnector::new(transport);

        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: Some("pw".to_string()),
        };

        // Act
        connector.handshake(request).expect("handshake failed");

        // Assert
        let sent = sent.borrow();
        assert_eq!(sent.len(), 1);
        assert!(matches!(
            &sent[0],
            ControlPacket::Authenticate(msg)
                if msg.username.as_deref() == Some("alice")
                    && msg.password.as_deref() == Some("pw")
        ));
    }

    /// Handshake maps known control packets into domain messages.
    #[test]
    fn handshake_maps_control_packets() {
        // Arrange
        let sent = Rc::new(RefCell::new(Vec::new()));
        let mut server_sync = msgs::ServerSync::new();
        server_sync.session = Some(7);

        let mut channel_state = msgs::ChannelState::new();
        channel_state.channel_id = Some(1);
        channel_state.name = Some("Lobby".to_string());

        let mut user_state = msgs::UserState::new();
        user_state.session = Some(2);
        user_state.name = Some("Alice".to_string());
        user_state.channel_id = Some(1);
        user_state.self_mute = Some(true);
        user_state.self_deaf = Some(false);

        let transport = TestTransport {
            sent: Rc::clone(&sent),
            recv_queue: vec![
                ControlPacket::ServerSync(Box::new(server_sync)),
                ControlPacket::ChannelState(Box::new(channel_state)),
                ControlPacket::UserState(Box::new(user_state)),
            ],
            send_error: false,
            recv_error: false,
        };
        let mut connector = MumbleProtocolControlConnector::new(transport);

        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let handshake = connector.handshake(request).expect("handshake failed");
        let messages = handshake.messages;
        // Assert
        assert_eq!(
            messages,
            vec![
                ControlMessage::ServerSync { session: 7 },
                ControlMessage::ChannelState {
                    id: 1,
                    name: "Lobby".to_string(),
                    parent_id: None,
                },
                ControlMessage::UserState {
                    id: 2,
                    name: "Alice".to_string(),
                    channel_id: 1,
                    muted: true,
                    deafened: false,
                    talking: false,
                },
            ]
        );
    }

    /// Handshake ignores control packets that are not mapped.
    #[test]
    fn handshake_ignores_unknown_packets() {
        // Arrange
        let sent = Rc::new(RefCell::new(Vec::new()));
        let ban_list = msgs::BanList::new();
        let transport = TestTransport {
            sent: Rc::clone(&sent),
            recv_queue: vec![ControlPacket::BanList(Box::new(ban_list))],
            send_error: false,
            recv_error: false,
        };
        let mut connector = MumbleProtocolControlConnector::new(transport);

        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let handshake = connector.handshake(request).expect("handshake failed");
        let messages = handshake.messages;
        // Assert
        assert!(messages.is_empty());
    }

    /// Handshake skips packets missing required fields so partial state does not leak.
    #[test]
    fn handshake_skips_incomplete_messages() {
        // Arrange
        let sent = Rc::new(RefCell::new(Vec::new()));
        let server_sync = msgs::ServerSync::new();
        let mut channel_state = msgs::ChannelState::new();
        channel_state.channel_id = Some(1);
        let mut user_state = msgs::UserState::new();
        user_state.session = Some(2);
        user_state.name = Some("Alice".to_string());

        let transport = TestTransport {
            sent: Rc::clone(&sent),
            recv_queue: vec![
                ControlPacket::ServerSync(Box::new(server_sync)),
                ControlPacket::ChannelState(Box::new(channel_state)),
                ControlPacket::UserState(Box::new(user_state)),
            ],
            send_error: false,
            recv_error: false,
        };
        let mut connector = MumbleProtocolControlConnector::new(transport);

        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let handshake = connector.handshake(request).expect("handshake failed");
        let messages = handshake.messages;
        // Assert
        assert!(messages.is_empty());
    }

    /// Handshake surfaces transport send failures instead of swallowing them.
    #[test]
    fn handshake_propagates_send_error() {
        // Arrange
        let transport = TestTransport {
            sent: Rc::new(RefCell::new(Vec::new())),
            recv_queue: Vec::new(),
            send_error: true,
            recv_error: false,
        };
        let mut connector = MumbleProtocolControlConnector::new(transport);
        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let err = connector
            .handshake(request)
            .expect_err("expected send failure");
        // Assert
        assert!(matches!(err, TransportError::Io(_)));
    }

    /// Handshake surfaces transport receive failures instead of continuing with stale state.
    #[test]
    fn handshake_propagates_recv_error() {
        // Arrange
        let transport = TestTransport {
            sent: Rc::new(RefCell::new(Vec::new())),
            recv_queue: Vec::new(),
            send_error: false,
            recv_error: true,
        };
        let mut connector = MumbleProtocolControlConnector::new(transport);
        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let err = connector
            .handshake(request)
            .expect_err("expected recv failure");
        // Assert
        assert!(matches!(err, TransportError::Io(_)));
    }

    /// No-op connector returns no messages on handshake.
    #[test]
    fn noop_connector_returns_empty_messages() {
        // Arrange
        let mut connector = super::NoopControlConnector;
        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let handshake = connector.handshake(request).expect("handshake failed");
        let messages = handshake.messages;
        // Assert
        assert!(messages.is_empty());
    }

    /// Blocking transport decodes a packet from bytes.
    #[test]
    fn blocking_transport_send_and_recv_roundtrip() {
        // Arrange
        let mut auth = msgs::Authenticate::new();
        auth.username = Some("alice".to_string());
        auth.password = Some("pw".to_string());

        let mut codec = mumble_protocol_2x::control::ClientControlCodec::new();
        let mut out = bytes::BytesMut::new();
        codec
            .encode(ControlPacket::Authenticate(Box::new(auth)), &mut out)
            .expect("encode failed");

        let cursor = Cursor::new(out.to_vec());
        let mut transport = BlockingControlTransport::new(cursor);
        // Act
        let packet = transport.recv().expect("recv failed").expect("no packet");

        // Assert
        assert!(matches!(packet, ControlPacket::Authenticate(_)));
    }

    /// Blocking transport encodes and writes packets to the stream.
    #[test]
    fn blocking_transport_send_writes_bytes() {
        // Arrange
        let cursor = Cursor::new(Vec::new());
        let mut transport = BlockingControlTransport::new(cursor);

        let mut auth = msgs::Authenticate::new();
        auth.username = Some("alice".to_string());
        // Act
        transport
            .send(ControlPacket::Authenticate(Box::new(auth)))
            .expect("send failed");

        let data = transport.into_inner().into_inner();
        // Assert
        assert!(!data.is_empty());

        let mut codec = mumble_protocol_2x::control::ClientControlCodec::new();
        let mut buffer = bytes::BytesMut::from(&data[..]);
        let decoded = codec
            .decode(&mut buffer)
            .expect("decode failed")
            .expect("missing packet");
        assert!(matches!(decoded, ControlPacket::Authenticate(_)));
    }

    /// EOF yields no packet instead of a decode error.
    #[test]
    fn blocking_transport_recv_empty_returns_none() {
        // Arrange
        let cursor = Cursor::new(Vec::new());
        let mut transport = BlockingControlTransport::new(cursor);
        // Act
        let packet = transport.recv().expect("recv failed");
        // Assert
        assert!(packet.is_none());
    }

    /// Socket connector wires the stream and returns mapped messages.
    #[test]
    fn socket_connector_builds_transport_and_returns_messages() {
        // Arrange
        let mut server_sync = msgs::ServerSync::new();
        server_sync.session = Some(9);

        let mut codec = mumble_protocol_2x::control::ClientControlCodec::new();
        let mut out = bytes::BytesMut::new();
        codec
            .encode(ControlPacket::ServerSync(Box::new(server_sync)), &mut out)
            .expect("encode failed");

        let captured = Rc::new(RefCell::new(None));
        let captured_clone = Rc::clone(&captured);
        let mut stream = Some(MemoryStream::with_read_data(out.to_vec()));

        let mut connector = SocketControlConnector::new(
            move |request: &HandshakeRequest| -> Result<MemoryStream, TransportError> {
                *captured_clone.borrow_mut() = Some(request.clone());
                Ok(stream.take().expect("stream already taken"))
            },
        );

        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let handshake = connector
            .handshake(request.clone())
            .expect("handshake failed");
        let messages = handshake.messages;
        // Assert
        assert_eq!(messages, vec![ControlMessage::ServerSync { session: 9 }]);
        assert_eq!(*captured.borrow(), Some(request));
    }

    /// Socket connector forwards connection failures to callers.
    #[test]
    fn socket_connector_propagates_connect_error() {
        // Arrange
        let mut connector = SocketControlConnector::new(
            |_: &HandshakeRequest| -> Result<MemoryStream, TransportError> {
                Err(TransportError::Io("connect failed".to_string()))
            },
        );

        let request = HandshakeRequest {
            server: "voice.example".to_string(),
            port: 64738,
            username: "alice".to_string(),
            password: None,
        };

        // Act
        let err = connector
            .handshake(request)
            .expect_err("expected connect failure");
        // Assert
        assert!(matches!(err, TransportError::Io(_)));
    }
}
