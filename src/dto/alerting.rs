use crate::error::CliError;
use crate::model::{AssignmentId, DestinationId, MonitorId, WorkspaceId};
use crate::wire::{Response, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A supported destination channel. Unimplemented channels are not exposed.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Email,
    Webhook,
    Telegram,
    Ntfy,
    Pushover,
    Discord,
    Slack,
}
impl ChannelKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Email => "Email",
            Self::Webhook => "Webhook",
            Self::Telegram => "Telegram",
            Self::Ntfy => "ntfy",
            Self::Pushover => "Pushover",
            Self::Discord => "Discord",
            Self::Slack => "Slack",
        }
    }
}
/// API-owned destination availability.
#[derive(Clone, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DestinationState {
    Available,
    Unavailable {
        plan_change_id: String,
        since: String,
    },
}
/// Safe destination metadata, retaining extensible configuration in explicit JSON.
#[derive(Clone, Deserialize)]
pub struct Destination {
    pub id: DestinationId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub channel_type: ChannelKind,
    pub state: DestinationState,
    pub config: Secret<Value>,
    pub created_at: String,
    pub updated_at: String,
}
/// One page with an explicit continuation cursor.
#[derive(Clone, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub next_cursor: Option<String>,
}
/// Monitor-to-destination binding returned by the public API.
#[derive(Clone, Deserialize)]
pub struct Assignment {
    pub id: AssignmentId,
    pub monitor_id: MonitorId,
    pub alert_target_id: DestinationId,
    pub workspace_id: WorkspaceId,
    pub created_at: String,
}
/// Immediate test result; accepted means the provider accepted delivery.
#[derive(Clone, Deserialize)]
pub struct DestinationTest {
    pub id: String,
    pub alert_target_id: DestinationId,
    pub channel_kind: ChannelKind,
    pub outcome: TestOutcome,
    pub attempted_at: String,
}
/// Provider-reported test outcome.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestOutcome {
    Accepted,
    Failed,
    Skipped,
}

/// A validated complete destination creation request with redacted diagnostics.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct CreateDestinationInput(Response<TargetRequest<CreateChannel>>);
/// A validated complete replacement with explicit secret preservation.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct UpdateDestinationInput(Response<TargetRequest<UpdateChannel>>);

macro_rules! input {
    ($name:ident, $message:literal) => {
        impl TryFrom<Value> for $name {
            type Error = CliError;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                Response::parse(value)
                    .map(Self)
                    .map_err(|_| CliError::InvalidInput($message.into()))
            }
        }
    };
}
input!(
    CreateDestinationInput,
    "destination creation JSON must include name, channel_type and that channel's required settings"
);
input!(
    UpdateDestinationInput,
    "destination update JSON must include a complete replacement with explicit secret actions"
);

#[derive(Clone, Deserialize, Serialize)]
struct TargetRequest<C> {
    name: String,
    #[serde(flatten)]
    channel: C,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "channel_type", rename_all = "snake_case")]
enum CreateChannel {
    Email {
        email_address: String,
    },
    Webhook {
        webhook_url: Secret<String>,
    },
    Telegram {
        telegram_bot_token: Secret<String>,
        telegram_chat_id: String,
    },
    Ntfy {
        ntfy_topic_url: Secret<String>,
        ntfy_priority: Option<NtfyPriority>,
    },
    Pushover {
        pushover_user_key: Secret<String>,
        pushover_app_token: Secret<String>,
        pushover_priority: Option<PushoverPriority>,
    },
    Discord {
        discord_webhook_url: Secret<String>,
    },
    Slack {
        slack_webhook_url: Secret<String>,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "channel_type", rename_all = "snake_case")]
enum UpdateChannel {
    Email {
        email_address: String,
    },
    Webhook {
        webhook_url: Secret<String>,
    },
    Telegram {
        telegram_bot_token: SecretChange,
        telegram_chat_id: String,
    },
    Ntfy {
        ntfy_topic_url: Secret<String>,
        ntfy_priority: NtfyPriority,
    },
    Pushover {
        pushover_user_key: SecretChange,
        pushover_app_token: SecretChange,
        pushover_priority: PushoverPriority,
    },
    Discord {
        discord_webhook_url: SecretChange,
    },
    Slack {
        slack_webhook_url: SecretChange,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum SecretChange {
    Preserve,
    Replace { value: Secret<String> },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum NtfyPriority {
    Min,
    Low,
    Default,
    High,
    Max,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum PushoverPriority {
    Lowest,
    Low,
    Normal,
    High,
    Emergency,
}

impl UpdateDestinationInput {
    /// Rename using the existing editable settings and explicit preservation of write-only secrets.
    pub fn rename(current: &Destination, name: String) -> Result<Self, CliError> {
        let mut body = current
            .config
            .expose()
            .as_object()
            .cloned()
            .ok_or_else(|| {
                CliError::MalformedResponse("destination configuration must be an object".into())
            })?;
        body.insert("name".into(), Value::String(name));
        body.insert(
            "channel_type".into(),
            serde_json::to_value(current.channel_type)
                .map_err(|_| CliError::InvalidInput("cannot encode channel type".into()))?,
        );
        let secrets: &[&str] = match current.channel_type {
            ChannelKind::Telegram => &["telegram_bot_token"],
            ChannelKind::Pushover => &["pushover_user_key", "pushover_app_token"],
            ChannelKind::Discord => &["discord_webhook_url"],
            ChannelKind::Slack => &["slack_webhook_url"],
            ChannelKind::Email | ChannelKind::Webhook | ChannelKind::Ntfy => &[],
        };
        body.retain(|key, _| !key.ends_with("_configured"));
        for field in secrets {
            body.insert((*field).into(), serde_json::json!({"action": "preserve"}));
        }
        Self::try_from(Value::Object(body)).map_err(|_| {
            CliError::MalformedResponse(
                "destination detail lacks editable settings; no update sent".into(),
            )
        })
    }
}

#[cfg(test)]
mod tests;
