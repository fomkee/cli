use super::*;
use serde_json::json;
fn assert_create(channel: Value) {
    let mut body = channel;
    body.as_object_mut()
        .unwrap()
        .insert("name".into(), json!("Ops"));
    let input = CreateDestinationInput::try_from(body.clone()).unwrap();
    assert_eq!(serde_json::to_value(&input).unwrap(), body);
    assert!(!format!("{input:?}").contains("private"));
}
#[test]
fn test_email_creation_shape() {
    assert_create(json!({"channel_type":"email","email_address":"ops@example.com"}));
}
#[test]
fn test_webhook_creation_shape() {
    assert_create(json!({"channel_type":"webhook","webhook_url":"https://example.com/private"}));
}
#[test]
fn test_telegram_creation_shape() {
    assert_create(
        json!({"channel_type":"telegram","telegram_bot_token":"private","telegram_chat_id":"123"}),
    );
}
#[test]
fn test_ntfy_creation_shape() {
    assert_create(
        json!({"channel_type":"ntfy","ntfy_topic_url":"https://ntfy.sh/private","ntfy_priority":"high"}),
    );
}
#[test]
fn test_pushover_creation_shape() {
    assert_create(
        json!({"channel_type":"pushover","pushover_user_key":"private","pushover_app_token":"private","pushover_priority":"normal"}),
    );
}
#[test]
fn test_discord_creation_shape() {
    assert_create(
        json!({"channel_type":"discord","discord_webhook_url":"https://discord.com/private"}),
    );
}
#[test]
fn test_slack_creation_shape() {
    assert_create(
        json!({"channel_type":"slack","slack_webhook_url":"https://hooks.slack.com/private"}),
    );
}
#[test]
fn test_rejects_unimplemented_channel() {
    assert!(
        CreateDestinationInput::try_from(json!({"name":"Ops","channel_type":"pager_duty"}))
            .is_err()
    );
}
#[test]
fn test_rejects_missing_required_secret() {
    assert!(
        CreateDestinationInput::try_from(
            json!({"name":"Ops","channel_type":"telegram","telegram_chat_id":"123"})
        )
        .is_err()
    );
}
#[test]
fn test_update_requires_explicit_secret_action() {
    assert!(
        UpdateDestinationInput::try_from(
            json!({"name":"Ops","channel_type":"slack","slack_webhook_url":"private"})
        )
        .is_err()
    );
}
#[test]
fn test_update_keeps_explicit_replace_action_and_extension_fields() {
    let body = json!({"name":"Ops","channel_type":"slack","slack_webhook_url":{"action":"replace","value":"private"},"extension":true});
    let input = UpdateDestinationInput::try_from(body.clone()).unwrap();
    assert_eq!(serde_json::to_value(&input).unwrap(), body);
    assert!(!format!("{input:?}").contains("private"));
}
fn assert_update(channel: Value) {
    let mut body = channel;
    body.as_object_mut()
        .unwrap()
        .insert("name".into(), json!("Ops"));
    let input = UpdateDestinationInput::try_from(body.clone()).unwrap();
    assert_eq!(serde_json::to_value(input).unwrap(), body);
}
#[test]
fn test_email_replacement_shape() {
    assert_update(json!({"channel_type":"email","email_address":"ops@example.com"}));
}
#[test]
fn test_webhook_replacement_shape() {
    assert_update(json!({"channel_type":"webhook","webhook_url":"https://example.com"}));
}
#[test]
fn test_telegram_replacement_shape() {
    assert_update(
        json!({"channel_type":"telegram","telegram_chat_id":"123","telegram_bot_token":{"action":"preserve"}}),
    );
}
#[test]
fn test_ntfy_replacement_shape() {
    assert_update(
        json!({"channel_type":"ntfy","ntfy_topic_url":"https://ntfy.sh/topic","ntfy_priority":"default"}),
    );
}
#[test]
fn test_pushover_replacement_shape() {
    assert_update(
        json!({"channel_type":"pushover","pushover_user_key":{"action":"preserve"},"pushover_app_token":{"action":"preserve"},"pushover_priority":"normal"}),
    );
}
#[test]
fn test_discord_replacement_shape() {
    assert_update(json!({"channel_type":"discord","discord_webhook_url":{"action":"preserve"}}));
}
