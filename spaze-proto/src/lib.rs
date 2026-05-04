//! `spaze-proto` — wire protocol types for Spaze.
//!
//! This crate intentionally contains no I/O and no business logic.
//! It defines IDs, messages, events, and protocol errors that all other
//! crates serialize over the wire.

pub mod error;
pub mod events;
pub mod ids;
pub mod messages;

pub use error::ProtocolError;
pub use events::{
    ClientCommand, ClientFrame, CommandOutcome, RequestId, ResponsePayload, ServerEvent,
    ServerFrame,
};
pub use ids::{DeviceId, MessageId, RoomId, SpaceId, UserId};
pub use messages::{Message, MessageBody};

/// Wire protocol version. Bumped when message/event shapes change incompatibly.
pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_per_call() {
        let a = UserId::new();
        let b = UserId::new();
        assert_ne!(a, b, "two fresh UserIds must not collide");
    }

    #[test]
    fn user_id_roundtrips_json() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn protocol_error_roundtrips_json() {
        let err = ProtocolError::NotFound {
            what: "channel".into(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"not_found\""), "tag missing in {json}");
        let parsed: ProtocolError = serde_json::from_str(&json).unwrap();
        match parsed {
            ProtocolError::NotFound { what } => assert_eq!(what, "channel"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn protocol_error_displays_human_readable() {
        let err = ProtocolError::RateLimited {
            retry_after_secs: 30,
        };
        assert_eq!(err.to_string(), "rate limited; retry after 30s");
    }

    #[test]
    fn message_roundtrips_json() {
        let msg = Message {
            id: MessageId::new(),
            room_id: RoomId::new(),
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            created_at_ms: 1_700_000_000_000,
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::Text {
                content: "hello world".into(),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, msg.id);
        assert_eq!(parsed.edited_at_ms, None);
        assert_eq!(parsed.deleted_at_ms, None);
        match parsed.body {
            MessageBody::Text { content } => assert_eq!(content, "hello world"),
            other @ MessageBody::System { .. } => panic!("wrong body variant: {other:?}"),
        }
    }

    #[test]
    fn edited_message_carries_edit_timestamp() {
        let msg = Message {
            id: MessageId::new(),
            room_id: RoomId::new(),
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            created_at_ms: 1_700_000_000_000,
            edited_at_ms: Some(1_700_000_500_000),
            deleted_at_ms: None,
            body: MessageBody::Text {
                content: "edited content".into(),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.edited_at_ms, Some(1_700_000_500_000));
    }

    #[test]
    fn system_message_serializes_with_tag() {
        let body = MessageBody::System {
            content: "user joined".into(),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(
            json.contains("\"kind\":\"system\""),
            "tag missing in {json}"
        );
    }

    #[test]
    fn server_event_roundtrips_json() {
        let event = ServerEvent::UserJoined {
            user_id: UserId::new(),
            room_id: RoomId::new(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("\"type\":\"user_joined\""),
            "tag missing in {json}"
        );
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ServerEvent::UserJoined { .. }));
    }

    #[test]
    fn server_event_message_posted_roundtrips() {
        let msg = Message {
            id: MessageId::new(),
            room_id: RoomId::new(),
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            created_at_ms: 1_700_000_000_000,
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::Text {
                content: "hello".into(),
            },
        };
        let event = ServerEvent::MessagePosted(msg.clone());
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("\"type\":\"message_posted\""),
            "type tag missing in {json}"
        );
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::MessagePosted(parsed_msg) => {
                assert_eq!(parsed_msg.id, msg.id);
                assert_eq!(parsed_msg.author_id, msg.author_id);
                match parsed_msg.body {
                    MessageBody::Text { content } => assert_eq!(content, "hello"),
                    other => panic!("wrong body variant: {other:?}"),
                }
            }
            other => panic!("wrong event variant: {other:?}"),
        }
    }

    #[test]
    fn client_frame_carries_request_id_and_command() {
        let frame = ClientFrame {
            request_id: RequestId(42),
            command: ClientCommand::PostMessage {
                room_id: RoomId::new(),
                body: MessageBody::Text {
                    content: "hi".into(),
                },
            },
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(
            json.contains("\"request_id\":42"),
            "request_id missing in {json}"
        );
        assert!(
            json.contains("\"type\":\"post_message\""),
            "command tag missing in {json}"
        );
        let parsed: ClientFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.request_id, RequestId(42));
        assert!(matches!(parsed.command, ClientCommand::PostMessage { .. }));
    }

    #[test]
    fn server_response_frame_roundtrips_ok_outcome() {
        let frame = ServerFrame::Response {
            request_id: RequestId(7),
            result: CommandOutcome::Ok {
                payload: ResponsePayload::Empty,
            },
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(
            json.contains("\"frame\":\"response\""),
            "outer tag missing in {json}"
        );
        assert!(
            json.contains("\"outcome\":\"ok\""),
            "outcome tag missing in {json}"
        );
        let parsed: ServerFrame = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerFrame::Response { request_id, result } => {
                assert_eq!(request_id, RequestId(7));
                assert!(matches!(result, CommandOutcome::Ok { .. }));
            }
            other @ ServerFrame::Event(_) => panic!("wrong frame variant: {other:?}"),
        }
    }

    #[test]
    fn server_response_frame_roundtrips_err_outcome() {
        let frame = ServerFrame::Response {
            request_id: RequestId(9),
            result: CommandOutcome::Err {
                error: ProtocolError::PermissionDenied,
            },
        };
        let json = serde_json::to_string(&frame).unwrap();
        let parsed: ServerFrame = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerFrame::Response {
                result: CommandOutcome::Err { error },
                ..
            } => {
                assert_eq!(error, ProtocolError::PermissionDenied);
            }
            other => panic!("expected Err outcome, got {other:?}"),
        }
    }

    #[test]
    fn server_event_frame_roundtrips() {
        let frame = ServerFrame::Event(ServerEvent::Typing {
            user_id: UserId::new(),
            room_id: RoomId::new(),
        });
        let json = serde_json::to_string(&frame).unwrap();
        assert!(
            json.contains("\"frame\":\"event\""),
            "outer tag missing in {json}"
        );
        assert!(
            json.contains("\"type\":\"typing\""),
            "event tag missing in {json}"
        );
        let parsed: ServerFrame = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            parsed,
            ServerFrame::Event(ServerEvent::Typing { .. })
        ));
    }
}
