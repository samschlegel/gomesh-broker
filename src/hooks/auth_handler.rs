/// Hook handler for MQTT client authentication.
///
/// Called by rmqtt when a client connects. Delegates to the
/// `Authenticator` trait and maps the result to an rmqtt hook response.
///
/// # Flow
/// 1. Extract username/password from the CONNECT packet.
/// 2. Call `authenticator.authenticate(username, password)`.
/// 3. On success, store the `ClientIdentity` in the session/client state.
/// 4. On denial, reject the connection with the appropriate CONNACK code.
pub fn handle_authenticate() {
    todo!("Implement rmqtt on_client_authenticate hook")
}
