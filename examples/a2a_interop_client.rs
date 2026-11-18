//! Interop probe: connect to an external A2A server (Google ADK) from
//! Synapse, reading its card and sending a message.
//!
//! Usage: cargo run --example a2a_interop_client -- <peer-base-url>

use synapse::a2a::client::A2AClient;
use synapse::schema::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let peer = std::env::args()
        .nth(1)
        .ok_or("usage: a2a_interop_client <peer-base-url>")?;

    let client = A2AClient::with_extensions(
        &peer,
        vec!["https://example.com/ext/v1".to_string()],
    )
    .await?;

    // 1. read the peer's card
    match client.get_agent_card().await {
        Ok(card) => {
            println!("PEER_CARD_NAME {}", card.name);
            println!("PEER_CARD_VERSION {}", card.version);
            println!("PEER_CARD_EXTENSIONS {:?}", card.capabilities.extensions);
        }
        Err(e) => println!("PEER_CARD_ERROR {e}"),
    }

    // 2. send a message and see what comes back
    let question = "interop probe";
    let reply = client.send_message(Message::user(question)).await;
    match reply {
        Ok(msg) => {
            let text = match &msg.content {
                synapse::schema::Content::Text(t) => t.clone(),
                _ => String::new(),
            };
            println!("PEER_REPLY {text}");
            // The answer must come from the peer's artifacts. If the
            // caller's own words come back, the peer either echoed the
            // request or the client read the wrong place -- both look
            // like a well-formed reply, so say so out loud.
            if text.trim() == question {
                println!("PEER_REPLY_IS_ECHO true");
            }
        }
        Err(e) => println!("PEER_REPLY_ERROR {e}"),
    }

    // 3. the same question over the streaming path
    //
    // The two paths answer differently, and both matter: the unary
    // path reads the answer out of the task's artifacts, the streaming
    // path reads it out of the artifact updates (accumulated, because a
    // peer may stream the answer in chunks).
    match client.send_message_stream(Message::user(question)).await {
        Ok(mut stream) => {
            let mut answer = String::new();
            let mut failure: Option<String> = None;
            while let Some(event) = stream.next().await {
                match event {
                    synapse::adk::agent::AgentStreamEvent::TokenDelta { delta, .. } => {
                        answer.push_str(&delta);
                    }
                    synapse::adk::agent::AgentStreamEvent::Final { message, .. } => {
                        if let synapse::schema::Content::Text(text) = &message.content {
                            // The final message is the authoritative
                            // answer: it carries the whole artifact,
                            // not just the last chunk.
                            answer = text.clone();
                        }
                    }
                    synapse::adk::agent::AgentStreamEvent::Error { error, .. } => {
                        failure = Some(format!("{error}"));
                    }
                    _ => {}
                }
            }
            match failure {
                Some(error) => println!("STREAM_ERROR {error}"),
                None => println!("STREAM_REPLY {answer}"),
            }
        }
        Err(e) => println!("STREAM_ERROR {e}"),
    }

    Ok(())
}
