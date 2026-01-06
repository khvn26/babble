use crate::mumble::state::StateCache;
#[cfg(not(feature = "coverage"))]
use crate::mumble::{tls_connect, SocketControlConnector};
use crate::mumble::{
    ControlConnector, ControlMessage, ControlSession, HandshakeRequest, MumbleConfig,
    NoopControlConnector, TransportEvent, UserStateCommand,
};
use crate::transport::errors::TransportError;
use crate::transport::types::ConnState;

pub struct MumbleTransport {
    config: MumbleConfig,
    conn_state: ConnState,
    events: Vec<TransportEvent>,
    control: Box<dyn ControlConnector>,
    state: StateCache,
    session_id: Option<u32>,
    current_channel_id: Option<u32>,
    control_session: Option<Box<dyn ControlSession>>,
}

impl MumbleTransport {
    pub fn new(config: MumbleConfig) -> Self {
        Self::with_connector(config, Box::new(NoopControlConnector))
    }

    #[cfg(not(feature = "coverage"))]
    pub fn new_with_tls(config: MumbleConfig) -> Self {
        let connector = SocketControlConnector::new(tls_connect);
        Self::with_connector(config, Box::new(connector))
    }

    pub fn with_connector(config: MumbleConfig, control: Box<dyn ControlConnector>) -> Self {
        Self {
            config,
            conn_state: ConnState::Disconnected,
            events: Vec::new(),
            control,
            state: StateCache::new(),
            session_id: None,
            current_channel_id: None,
            control_session: None,
        }
    }

    pub fn conn_state(&self) -> ConnState {
        self.conn_state
    }

    pub fn take_events(&mut self) -> Vec<TransportEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn session_id(&self) -> Option<u32> {
        self.session_id
    }

    pub fn current_channel_id(&self) -> Option<u32> {
        self.current_channel_id
    }

    pub fn connect(&mut self) -> Result<(), TransportError> {
        if self.conn_state != ConnState::Disconnected {
            return Ok(());
        }

        let server = self.config.server.trim();
        if server.is_empty() {
            return Err(TransportError::InvalidConfig(
                "server is required".to_string(),
            ));
        }
        let username = self.config.username.trim();
        if username.is_empty() {
            return Err(TransportError::InvalidConfig(
                "username is required".to_string(),
            ));
        }

        self.set_conn_state(ConnState::Connecting);
        let request = HandshakeRequest {
            server: self.config.server.clone(),
            port: self.config.port,
            username: self.config.username.clone(),
            password: self.config.password.clone(),
        };
        let handshake = match self.control.handshake(request) {
            Ok(handshake) => handshake,
            Err(error) => {
                self.set_conn_state(ConnState::Error);
                self.events.push(TransportEvent::Error(error.to_string()));
                return Err(error);
            }
        };
        self.control_session = handshake.session;

        for message in handshake.messages {
            self.apply_control_message(message);
        }

        self.set_conn_state(ConnState::Connected);
        Ok(())
    }

    pub fn join_channel(&mut self, channel_id: u32) -> Result<(), TransportError> {
        if self.conn_state != ConnState::Connected {
            return Err(TransportError::Disconnected);
        }

        let session_id = self
            .session_id
            .ok_or_else(|| TransportError::Protocol("missing session id".to_string()))?;

        if self.state.channel(channel_id).is_none() {
            return Err(TransportError::Protocol("unknown channel".to_string()));
        }

        if self.state.user(session_id).is_none() {
            return Err(TransportError::Protocol(
                "missing self user state".to_string(),
            ));
        }

        let session = self
            .control_session
            .as_mut()
            .ok_or_else(|| TransportError::Protocol("control session unavailable".to_string()))?;
        session.send_user_state(UserStateCommand {
            session_id,
            channel_id,
            muted: None,
            deafened: None,
        })?;

        self.state
            .apply_user_state(crate::mumble::state::UserStateUpdate {
                id: session_id,
                name: None,
                channel_id: Some(channel_id),
                muted: None,
                deafened: None,
                talking: None,
            });
        self.current_channel_id = Some(channel_id);
        let users = self.state.users();
        self.events.push(TransportEvent::Users(users));
        Ok(())
    }

    fn set_conn_state(&mut self, next: ConnState) {
        self.conn_state = next;
        self.events.push(TransportEvent::ConnectionState(next));
    }

    fn apply_control_message(&mut self, message: ControlMessage) {
        match message {
            ControlMessage::ServerSync { session } => {
                self.session_id = Some(session);
            }
            ControlMessage::ChannelState {
                id,
                name,
                parent_id,
            } => {
                self.state
                    .apply_channel_state(crate::mumble::state::ChannelStateUpdate {
                        id,
                        name: Some(name),
                        parent_id,
                    });
                let channels = self.state.channels();
                self.events.push(TransportEvent::Channels(channels));
            }
            ControlMessage::UserState {
                id,
                name,
                channel_id,
                muted,
                deafened,
                talking,
            } => {
                if self.session_id == Some(id) {
                    self.current_channel_id = Some(channel_id);
                }
                self.state
                    .apply_user_state(crate::mumble::state::UserStateUpdate {
                        id,
                        name: Some(name),
                        channel_id: Some(channel_id),
                        muted: Some(muted),
                        deafened: Some(deafened),
                        talking: Some(talking),
                    });
                let users = self.state.users();
                self.events.push(TransportEvent::Users(users));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MumbleTransport;
    use crate::mumble::config::DEFAULT_PORT;
    use crate::mumble::{
        ControlConnector, ControlHandshake, ControlMessage, ControlSession, HandshakeRequest,
        MumbleConfig, UserStateCommand,
    };
    use crate::transport::errors::TransportError;
    use crate::transport::types::ConnState;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct TestControlConnector {
        last_request: Rc<RefCell<Option<HandshakeRequest>>>,
        fail: bool,
    }

    impl ControlConnector for TestControlConnector {
        fn handshake(
            &mut self,
            request: HandshakeRequest,
        ) -> Result<ControlHandshake, TransportError> {
            *self.last_request.borrow_mut() = Some(request);
            if self.fail {
                return Err(TransportError::Protocol("handshake failed".to_string()));
            }
            Ok(ControlHandshake {
                messages: Vec::new(),
                session: None,
            })
        }
    }

    /// Connect transitions through connecting and connected states.
    #[test]
    fn connect_transitions_state_and_emits_events() {
        // Arrange
        let config = MumbleConfig::new("localhost".to_string(), DEFAULT_PORT, "tester".to_string());
        let mut transport = MumbleTransport::new(config);

        // Act
        transport.connect().expect("connect failed");

        // Assert
        assert_eq!(transport.conn_state(), ConnState::Connected);
        let events = transport.take_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            super::TransportEvent::ConnectionState(ConnState::Connecting)
        ));
        assert!(matches!(
            events[1],
            super::TransportEvent::ConnectionState(ConnState::Connected)
        ));
    }

    /// Repeated connect calls are no-ops after the first connection.
    #[test]
    fn connect_is_idempotent() {
        // Arrange
        let config = MumbleConfig::new("localhost".to_string(), DEFAULT_PORT, "tester".to_string());
        let mut transport = MumbleTransport::new(config);

        // Act
        transport.connect().expect("connect failed");
        transport.take_events();

        transport.connect().expect("second connect failed");
        // Assert
        assert!(transport.take_events().is_empty());
    }

    /// take_events drains the event queue after connect.
    #[test]
    fn take_events_drains_after_connect() {
        // Arrange
        let config = MumbleConfig::new("server".to_string(), DEFAULT_PORT, "tester".to_string());
        let mut transport = MumbleTransport::new(config);

        // Act
        transport.connect().expect("connect failed");
        // Assert
        assert_eq!(transport.take_events().len(), 2);
        assert!(transport.take_events().is_empty());
    }

    /// Connect rejects blank server values.
    #[test]
    fn connect_rejects_empty_server() {
        // Arrange
        let config = MumbleConfig::new("".to_string(), DEFAULT_PORT, "tester".to_string());
        let mut transport = MumbleTransport::new(config);

        // Act
        let err = transport.connect().expect_err("expected connect to fail");
        // Assert
        assert!(matches!(err, TransportError::InvalidConfig(_)));
    }

    /// Connect rejects blank username values.
    #[test]
    fn connect_rejects_empty_username() {
        // Arrange
        let config = MumbleConfig::new("server".to_string(), DEFAULT_PORT, "".to_string());
        let mut transport = MumbleTransport::new(config);

        // Act
        let err = transport.connect().expect_err("expected connect to fail");
        // Assert
        assert!(matches!(err, TransportError::InvalidConfig(_)));
    }

    /// Connect sends the expected handshake request to the connector.
    #[test]
    fn connect_sends_handshake_request() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let connector = TestControlConnector {
            last_request: Rc::clone(&capture),
            fail: false,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));

        // Act
        transport.connect().expect("connect failed");

        // Assert
        let request = capture.borrow().clone().expect("missing request");
        assert_eq!(
            request,
            HandshakeRequest {
                server: "voice.example".to_string(),
                port: DEFAULT_PORT,
                username: "tester".to_string(),
                password: None,
            }
        );
    }

    /// Handshake failure transitions to error state and emits error events.
    #[test]
    fn connect_emits_error_on_handshake_failure() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let connector = TestControlConnector {
            last_request: Rc::clone(&capture),
            fail: true,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));

        // Act
        let err = transport.connect().expect_err("expected connect to fail");
        // Assert
        assert!(matches!(err, TransportError::Protocol(_)));

        let events = transport.take_events();
        assert!(matches!(
            events.as_slice(),
            [
                super::TransportEvent::ConnectionState(ConnState::Connecting),
                super::TransportEvent::ConnectionState(ConnState::Error),
                super::TransportEvent::Error(_),
            ]
        ));
    }

    /// Server sync control messages update the stored session id.
    #[test]
    fn connect_applies_server_sync() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![ControlMessage::ServerSync { session: 42 }];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));

        // Act
        transport.connect().expect("connect failed");

        // Assert
        assert_eq!(transport.session_id(), Some(42));
    }

    /// Channel state messages update cached channels and emit events.
    #[test]
    fn connect_applies_channel_state() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![ControlMessage::ChannelState {
            id: 1,
            name: "Lobby".to_string(),
            parent_id: None,
        }];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));

        // Act
        transport.connect().expect("connect failed");

        // Assert
        let events = transport
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                super::TransportEvent::Channels(channels) => Some(channels),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0][0].name, "Lobby");
    }

    /// User state messages update cached users and emit events.
    #[test]
    fn connect_applies_user_state() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![ControlMessage::UserState {
            id: 42,
            name: "Alice".to_string(),
            channel_id: 1,
            muted: false,
            deafened: false,
            talking: true,
        }];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));

        // Act
        transport.connect().expect("connect failed");

        // Assert
        let events = transport
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                super::TransportEvent::Users(users) => Some(users),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0][0].name, "Alice");
        assert!(events[0][0].talking);
    }

    /// Server sync plus self user state updates the current channel id.
    #[test]
    fn connect_sets_current_channel_for_self() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![
            ControlMessage::ServerSync { session: 7 },
            ControlMessage::UserState {
                id: 7,
                name: "Self".to_string(),
                channel_id: 2,
                muted: false,
                deafened: false,
                talking: false,
            },
        ];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));

        // Act
        transport.connect().expect("connect failed");

        // Assert
        assert_eq!(transport.current_channel_id(), Some(2));
    }

    /// Join fails when the transport is disconnected.
    #[test]
    fn join_channel_rejects_when_disconnected() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let mut transport = MumbleTransport::new(config);

        // Act
        let err = transport
            .join_channel(1)
            .expect_err("expected join to fail");
        // Assert
        assert!(matches!(err, TransportError::Disconnected));
    }

    /// Join fails when session id is missing after connection.
    #[test]
    fn join_channel_rejects_missing_session() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let mut transport = MumbleTransport::new(config);
        transport.connect().expect("connect failed");
        transport.take_events();

        // Act
        let err = transport
            .join_channel(1)
            .expect_err("expected join to fail");
        // Assert
        assert!(matches!(err, TransportError::Protocol(_)));
        assert!(transport.take_events().is_empty());
    }

    /// Join fails when the target channel is not in the cache.
    #[test]
    fn join_channel_rejects_unknown_channel() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![
            ControlMessage::ServerSync { session: 42 },
            ControlMessage::UserState {
                id: 42,
                name: "Self".to_string(),
                channel_id: 1,
                muted: false,
                deafened: false,
                talking: false,
            },
            ControlMessage::ChannelState {
                id: 1,
                name: "Lobby".to_string(),
                parent_id: None,
            },
        ];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));
        transport.connect().expect("connect failed");
        transport.take_events();
        transport.take_events();

        // Act
        let err = transport
            .join_channel(99)
            .expect_err("expected join to fail");
        // Assert
        assert!(matches!(err, TransportError::Protocol(_)));
        assert!(transport.take_events().is_empty());
    }

    /// Join fails when the self user state is missing in the cache.
    #[test]
    fn join_channel_rejects_missing_self_user() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![
            ControlMessage::ServerSync { session: 7 },
            ControlMessage::ChannelState {
                id: 2,
                name: "Ops".to_string(),
                parent_id: None,
            },
        ];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));
        transport.connect().expect("connect failed");
        transport.take_events();

        // Act
        let err = transport
            .join_channel(2)
            .expect_err("expected join to fail");
        // Assert
        assert!(matches!(err, TransportError::Protocol(_)));
        assert!(transport.take_events().is_empty());
    }

    /// Join updates the cached self user channel and emits a user snapshot.
    #[test]
    fn join_channel_updates_self_channel() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let commands = Rc::new(RefCell::new(Vec::new()));
        let messages = vec![
            ControlMessage::ServerSync { session: 7 },
            ControlMessage::UserState {
                id: 7,
                name: "Self".to_string(),
                channel_id: 1,
                muted: false,
                deafened: false,
                talking: false,
            },
            ControlMessage::ChannelState {
                id: 1,
                name: "Lobby".to_string(),
                parent_id: None,
            },
            ControlMessage::ChannelState {
                id: 2,
                name: "Ops".to_string(),
                parent_id: None,
            },
        ];
        let connector = TestControlConnectorWithSession {
            last_request: Rc::clone(&capture),
            messages,
            session: TestControlSession::new(Rc::clone(&commands)),
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));
        transport.connect().expect("connect failed");

        // Act
        transport.join_channel(2).expect("join failed");

        // Assert
        assert_eq!(transport.current_channel_id(), Some(2));
        let users_events = transport
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                super::TransportEvent::Users(users) => Some(users),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(users_events.len(), 2);
        assert_eq!(users_events[0][0].channel_id, 1);
        assert_eq!(users_events[1][0].channel_id, 2);
    }

    /// Join fails when the control session is not available.
    #[test]
    fn join_channel_rejects_missing_control_session() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let messages = vec![
            ControlMessage::ServerSync { session: 7 },
            ControlMessage::UserState {
                id: 7,
                name: "Self".to_string(),
                channel_id: 1,
                muted: false,
                deafened: false,
                talking: false,
            },
            ControlMessage::ChannelState {
                id: 2,
                name: "Ops".to_string(),
                parent_id: None,
            },
        ];
        let connector = TestControlConnectorWithMessages {
            last_request: Rc::clone(&capture),
            messages,
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));
        transport.connect().expect("connect failed");
        transport.take_events();

        // Act
        let err = transport
            .join_channel(2)
            .expect_err("expected join to fail");
        // Assert
        assert!(matches!(err, TransportError::Protocol(_)));
        assert!(transport.take_events().is_empty());
    }

    /// Join sends the expected user state command to the control session.
    #[test]
    fn join_channel_sends_user_state_command() {
        // Arrange
        let config = MumbleConfig::new(
            "voice.example".to_string(),
            DEFAULT_PORT,
            "tester".to_string(),
        );
        let capture = Rc::new(RefCell::new(None));
        let commands = Rc::new(RefCell::new(Vec::new()));
        let messages = vec![
            ControlMessage::ServerSync { session: 7 },
            ControlMessage::UserState {
                id: 7,
                name: "Self".to_string(),
                channel_id: 1,
                muted: false,
                deafened: false,
                talking: false,
            },
            ControlMessage::ChannelState {
                id: 1,
                name: "Lobby".to_string(),
                parent_id: None,
            },
            ControlMessage::ChannelState {
                id: 2,
                name: "Ops".to_string(),
                parent_id: None,
            },
        ];
        let connector = TestControlConnectorWithSession {
            last_request: Rc::clone(&capture),
            messages,
            session: TestControlSession::new(Rc::clone(&commands)),
        };
        let mut transport = MumbleTransport::with_connector(config, Box::new(connector));
        transport.connect().expect("connect failed");
        transport.take_events();

        // Act
        transport.join_channel(2).expect("join failed");

        // Assert
        let commands = commands.borrow();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0],
            UserStateCommand {
                session_id: 7,
                channel_id: 2,
                muted: None,
                deafened: None,
            }
        );
    }

    struct TestControlConnectorWithMessages {
        last_request: Rc<RefCell<Option<HandshakeRequest>>>,
        messages: Vec<ControlMessage>,
    }

    struct TestControlConnectorWithSession {
        last_request: Rc<RefCell<Option<HandshakeRequest>>>,
        messages: Vec<ControlMessage>,
        session: TestControlSession,
    }

    struct TestControlSession {
        commands: Rc<RefCell<Vec<UserStateCommand>>>,
        fail: bool,
    }

    impl TestControlSession {
        fn new(commands: Rc<RefCell<Vec<UserStateCommand>>>) -> Self {
            Self {
                commands,
                fail: false,
            }
        }
    }

    impl ControlConnector for TestControlConnectorWithMessages {
        fn handshake(
            &mut self,
            request: HandshakeRequest,
        ) -> Result<ControlHandshake, TransportError> {
            *self.last_request.borrow_mut() = Some(request);
            Ok(ControlHandshake {
                messages: self.messages.clone(),
                session: None,
            })
        }
    }

    impl ControlConnector for TestControlConnectorWithSession {
        fn handshake(
            &mut self,
            request: HandshakeRequest,
        ) -> Result<ControlHandshake, TransportError> {
            *self.last_request.borrow_mut() = Some(request);
            Ok(ControlHandshake {
                messages: self.messages.clone(),
                session: Some(Box::new(TestControlSession {
                    commands: Rc::clone(&self.session.commands),
                    fail: self.session.fail,
                })),
            })
        }
    }

    impl ControlSession for TestControlSession {
        fn send_user_state(&mut self, command: UserStateCommand) -> Result<(), TransportError> {
            if self.fail {
                return Err(TransportError::Protocol("send failed".to_string()));
            }
            self.commands.borrow_mut().push(command);
            Ok(())
        }
    }
}
