use core::fmt;
use once_cell::sync::Lazy;
use regex::Regex;

static BRANCH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?P<kind>\w+)[/-])?(?P<team>\w+)-(?P<id>\d+).*$")
        .expect("could not compile story id regex")
});
static MESSAGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?P<kind>\w+): *)?(?:(?P<team>\w+)-(?P<id>\d+) +)?(?P<body>.*)$")
        .expect("could not compile message regex")
});

#[derive(thiserror::Error, Debug)]
pub enum MessageKindError {
    #[error("Invalid kind")]
    InvalidKind,
}

#[derive(thiserror::Error, Debug)]
pub enum MessageError {
    #[error("Invalid commit message")]
    InvalidCommitMessage,
    #[error("Invalid message kind (chore, fix, feat)")]
    InvalidKind(#[from] MessageKindError),
    #[error("Invalid commit message")]
    ParseError(#[from] regex::Error),
}

#[derive(Debug, Clone, Copy)]
pub enum MessageKind {
    Feature,
    Fix,
    Chore,
}

#[derive(Debug)]
struct StoryId {
    team: String,
    id: u32,
}

#[derive(Debug)]
pub struct Message {
    kind: MessageKind,
    story: StoryId,
    body: String,
}

impl MessageKind {
    fn new(kind: &str) -> Result<MessageKind, MessageKindError> {
        match kind {
            "feat" => Ok(MessageKind::Feature),
            "fix" => Ok(MessageKind::Fix),
            "chore" => Ok(MessageKind::Chore),
            _ => Err(MessageKindError::InvalidKind),
        }
    }
}

impl fmt::Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageKind::Feature => write!(f, "feat"),
            MessageKind::Fix => write!(f, "fix"),
            MessageKind::Chore => write!(f, "chore"),
        }
    }
}

impl StoryId {
    fn new(team: &str, id: u32) -> StoryId {
        StoryId {
            team: team.to_string(),
            id,
        }
    }
}

impl fmt::Display for StoryId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.team, self.id)
    }
}

impl Message {
    pub fn parse_message(
        message: &str,
        default_kind: Option<MessageKind>,
    ) -> Result<Message, MessageError> {
        match MESSAGE_REGEX.captures(message) {
            Some(capture) => {
                let kind = capture.name("kind").map(|kind| kind.as_str()).unwrap_or("");
                let team = capture.name("team").map(|team| team.as_str());
                let id: Option<u32> = capture.name("id").map(|id| id.as_str().parse().unwrap());
                let body = capture.name("body").map(|body| body.as_str());

                if team.is_none() || id.is_none() || body.is_none() {
                    return Err(MessageError::InvalidCommitMessage);
                }

                Ok(Message {
                    kind: MessageKind::new(kind)
                        .unwrap_or(default_kind.unwrap_or(MessageKind::Feature)),
                    story: StoryId::new(team.unwrap(), id.unwrap()),
                    body: body.unwrap().to_string(),
                })
            }
            None => Err(MessageError::InvalidCommitMessage),
        }
    }

    pub fn parse_branch(
        branch: &str,
        force_kind: Option<MessageKind>,
    ) -> Result<Message, MessageError> {
        match BRANCH_REGEX.captures(branch) {
            Some(capture) => {
                let team = capture.name("team").unwrap().as_str();
                let id = capture.name("id").unwrap().as_str().parse().unwrap();

                let kind = match capture.name("kind") {
                    Some(kind) => match force_kind {
                        Some(forced) => forced,
                        None => MessageKind::new(kind.as_str())?,
                    },
                    None => MessageKind::Feature,
                };

                Ok(Message {
                    kind,
                    story: StoryId::new(team, id),
                    body: "".to_string(),
                })
            }
            None => Err(MessageError::InvalidCommitMessage),
        }
    }

    pub fn parse(
        message: &str,
        branch: &str,
        kind: Option<MessageKind>,
    ) -> Result<Message, MessageError> {
        Message::parse_message(message, kind).or_else(|_| {
            Ok(Message {
                body: message.to_string(),
                ..Message::parse_branch(branch, kind)?
            })
        })
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {} {}", self.kind, self.story, self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_kind() {
        let feature = MessageKind::new("feat");
        let fix = MessageKind::new("fix");
        let chore = MessageKind::new("chore");
        let unknown = MessageKind::new("unknown");

        assert_eq!(feature.unwrap().to_string(), "feat");
        assert_eq!(fix.unwrap().to_string(), "fix");
        assert_eq!(chore.unwrap().to_string(), "chore");
        assert!(unknown.is_err());
    }

    #[test]
    fn test_parse_branch() {
        let branch = Message::parse_branch("team-123", None);
        assert_eq!(branch.unwrap().to_string(), "feat: team-123 ");

        let branch = Message::parse_branch("fix/team-123", None);
        assert_eq!(branch.unwrap().to_string(), "fix: team-123 ");

        let long_branch = Message::parse_branch("team-123-something-else", None);
        assert_eq!(long_branch.unwrap().to_string(), "feat: team-123 ");

        let long_branch = Message::parse_branch("chore/team-123-something-else", None);
        assert_eq!(long_branch.unwrap().to_string(), "chore: team-123 ");

        let long_branch =
            Message::parse_branch("chore/team-123-something-else", Some(MessageKind::Fix));
        assert_eq!(long_branch.unwrap().to_string(), "fix: team-123 ");

        let wildly_invalid = Message::parse_branch("just a string of some sort", None);
        assert!(wildly_invalid.is_err());

        let invalid = Message::parse_branch("123-something-else", None);
        assert!(invalid.is_err());

        let invalid = Message::parse_branch("flag/something-123-else", None);
        assert!(invalid.is_err());
    }

    #[test]
    fn test_parse_message() {
        let message = Message::parse_message("feat: team-123 something", None);
        assert_eq!(message.unwrap().to_string(), "feat: team-123 something");

        let message = Message::parse_message("fix: team-123 something", None);
        assert_eq!(message.unwrap().to_string(), "fix: team-123 something");

        let message = Message::parse_message("team-123 something", None);
        assert_eq!(message.unwrap().to_string(), "feat: team-123 something");

        let message = Message::parse_message("chore:  something", None);
        assert!(message.is_err());

        let message = Message::parse_message("team-abc something", None);
        assert!(message.is_err());

        let message = Message::parse_message("something", None);
        assert!(message.is_err());
    }

    #[test]
    fn test_parse() {
        let message = Message::parse("feat: team-123 something", "", None);
        assert_eq!(message.unwrap().to_string(), "feat: team-123 something");

        let message = Message::parse("something", "fix/team-123-blahblah", None);
        assert_eq!(message.unwrap().to_string(), "fix: team-123 something");

        let message = Message::parse("chore: something", "fix/team-123-h", None);
        assert_eq!(
            message.unwrap().to_string(),
            "fix: team-123 chore: something"
        );

        let message = Message::parse(
            "chore: something",
            "fix/team-123-h",
            Some(MessageKind::Chore),
        );
        assert_eq!(
            message.unwrap().to_string(),
            "chore: team-123 chore: something"
        );

        let message = Message::parse("something", "", None);
        assert!(message.is_err());
    }
}
