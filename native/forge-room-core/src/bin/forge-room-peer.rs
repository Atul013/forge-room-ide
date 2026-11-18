//! Headless native client for reproducible independent-process transport trials.
//! Invite and commands travel on stdin, never command-line arguments or disk.
use forge_room_core::transport::{Client, Invite};
use std::io::{self, BufRead, Read, Write};

fn main() {
    let mut client: Option<Client> = None;
    let mut hosted: Option<forge_room_core::transport::Host> = None;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = String::new();
        // Bound input before parsing, including malformed diagnostic commands.
        let read = (&mut input).take(1024 * 1024 + 1).read_line(&mut line);
        if matches!(read, Ok(0)) {
            break;
        }
        if read.is_err() || line.len() > 1024 * 1024 {
            break;
        }
        let result = (|| -> Result<serde_json::Value, String> {
            let value: serde_json::Value =
                serde_json::from_str(&line).map_err(|_| "Invalid command")?;
            match value["command"].as_str().unwrap_or_default() {
                "host_journal" => {
                    if hosted.is_some() {
                        return Err("Already hosting".into());
                    }
                    let host = forge_room_core::transport::Host::start(
                        "127.0.0.1:0".parse().map_err(|_| "Invalid address")?,
                        "process journal",
                    )?;
                    host.enable_journal(std::path::Path::new(
                        value["path"].as_str().ok_or("Missing journal path")?,
                    ))?;
                    let invite = host.invite(std::time::Duration::from_secs(60))?.encode();
                    hosted = Some(host);
                    Ok(serde_json::json!({"invite":invite}))
                }
                "join" => {
                    if client.is_some() {
                        return Err("Already joined".into());
                    }
                    let invite = Invite::decode(value["invite"].as_str().ok_or("Missing invite")?)?;
                    let (joined, snapshot) =
                        Client::join(invite, value["name"].as_str().ok_or("Missing name")?)?;
                    let identity = joined.identity.clone();
                    client = Some(joined);
                    Ok(serde_json::json!({"identity":identity,"snapshot":snapshot}))
                }
                "poll" => {
                    Ok(serde_json::json!({"snapshot":client.as_mut().ok_or("Not joined")?.poll()?}))
                }
                "post" => Ok(
                    serde_json::json!({"snapshot":client.as_mut().ok_or("Not joined")?.post(value["text"].as_str().ok_or("Missing text")?)?}),
                ),
                "execute" => {
                    let action = serde_json::from_value(value["action"].clone())
                        .map_err(|_| "Invalid action")?;
                    let outcome = client.as_mut().ok_or("Not joined")?.execute(action)?;
                    Ok(
                        serde_json::json!({"snapshot":outcome.snapshot,"rejection":outcome.rejection}),
                    )
                }
                "leave" => {
                    client.as_mut().ok_or("Not joined")?.leave()?;
                    client = None;
                    Ok(serde_json::json!({"left":true}))
                }
                _ => Err("Unknown command".into()),
            }
        })();
        let output = match result {
            Ok(value) => value,
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("{output}");
        if io::stdout().flush().is_err() {
            break;
        }
    }
    if let Some(mut client) = client {
        let _ = client.leave();
    }
}
