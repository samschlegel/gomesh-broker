/// Hook handler for MQTT subscribe authorization.
///
/// Called by rmqtt when a client subscribes to a topic filter. Delegates
/// to the `Authorizer` trait to check whether the client is allowed to
/// subscribe.
///
/// # Flow
/// 1. Retrieve the `ClientIdentity` from the session state.
/// 2. Call `authorizer.check(identity, TopicAction::Subscribe, topic)`.
/// 3. On `Allow`, grant the subscription.
/// 4. On `Deny`, reject with the appropriate SUBACK reason code.
pub fn handle_subscribe() {
    todo!("Implement rmqtt on_client_subscribe hook")
}
