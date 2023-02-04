use crossbeam::channel::Sender;
use futures_util::StreamExt;
use std::error::Error;
use tokio::net;
use tokio_tungstenite::{accept_async, tungstenite};

pub enum Message {
    Connected,
    Disconnected,
    AngleDiff(f32),
}

pub fn listen_for_phone(channel: Sender<Message>) {
    std::thread::spawn(move || {
        if let Err(err) = run_listening_task(channel) {
            eprintln!("{err}");
        };
    });
}

fn run_listening_task(channel: Sender<Message>) -> Result<(), Box<dyn Error>> {
    tokio::runtime::Runtime::new()?.block_on(async move {
        if let Err(err) = handle_messages(channel).await {
            eprintln!("{err}");
        }
    });
    Ok(())
}

async fn handle_messages(channel: Sender<Message>) -> Result<(), Box<dyn Error>> {
    loop {
        let (stream, _) = net::TcpListener::bind("0.0.0.0:8093")
            .await?
            .accept()
            .await?;

        let sink = accept_async(stream).await?;
        println!("it has connected");
        channel.try_send(Message::Connected)?;
        sink.for_each(|message| async {
            match handle_message(message).await {
                Ok(angle) => {
                    channel.try_send(Message::AngleDiff(angle * 2.0));
                }
                Err(err) => eprintln!("{err}"),
            };
        })
        .await;
        println!("nope");
        channel.try_send(Message::Disconnected)?;
    }
}

async fn handle_message(
    message: Result<tungstenite::Message, tungstenite::Error>,
) -> Result<f32, Box<dyn Error>> {
    Ok(message?.into_text()?.parse()?)
}
