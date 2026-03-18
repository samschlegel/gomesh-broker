/// Hook handler for MQTT publish authorization.
///
/// Called by rmqtt when a client publishes a message. Delegates to the
/// `Authorizer` trait to check whether the client is allowed to publish
/// to the given topic.
///
/// # Flow
/// 1. Retrieve the `ClientIdentity` from the session state.
/// 2. Call `authorizer.check(identity, TopicAction::Publish, topic)`.
/// 3. On `Allow` or `AllowStripRetain`, permit the publish (stripping retain if needed).
/// 4. On `Deny`, drop the message and optionally disconnect the client.
pub fn handle_publish() {
    todo!("Implement rmqtt on_message_publish hook")
}
