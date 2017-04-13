#[macro_use]
extern crate log;
extern crate env_logger;
extern crate irc;
extern crate github;

use irc::client::prelude::*;

fn main() {
    env_logger::init().unwrap();

    // This could be in a JSON config, but then we need to figure out how
    // to find that JSON config
    let irc_config: Config = Config {
        owners: Some(vec![format!("dbaron")]),
        nickname: Some(format!("wgmeeting-github-bot")),
        alt_nicks: Some(vec![format!("wgmeeting-github-bot-"),
                             format!("wgmeeting-github-bot--")]),
        username: Some(format!("dbaron-gh-bot")),
        realname: Some(format!("Bot to add meeting minutes to github issues.")),
        server: Some(format!("irc.w3.org")),
        port: Some(6667),
        use_ssl: Some(false),
        encoding: Some(format!("UTF-8")),
        channels: Some(vec![format!("#cssbottest")]),
        user_info: Some(format!("Bot to add meeting minutes to github issues.")),
        // FIXME: why doesn't this work as documented?
        //source: Some(format!("https://github.com/dbaron/wgmeeting-github-ircbot")),
        ..Default::default()
    };

    // FIXME: Eventually this should support multiple channels, plus
    // options to ask the bot which channels it's in, and which channels
    // it currently has buffers in.  (Then we can do things like ask the
    // bot to reboot itself, but it will only do so if it's not busy.)
    let mut channel_data = ChannelData::new();

    let server = IrcServer::from_config(irc_config).unwrap();
    server.identify().unwrap();
    for message in server.iter() {
        let message = message.unwrap(); // panic if there's an error

        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                match message.source_nickname() {
                    None => {
                        // FIXME: trailing \n
                        warn!("PRIVMSG without a source! {}", message);
                    }
                    Some(ref source) => {
                        let mynick = server.current_nickname();
                        if target == mynick {
                            handle_bot_command(&server, msg, source, None)
                        } else if target.starts_with('#') {
                            let source_ = String::from(*source);
                            let line = if msg.starts_with("\x01ACTION ") && msg.ends_with("\x01") {
                                ChannelLine {
                                    source: source_,
                                    is_action: true,
                                    message: String::from(&msg[8..msg.len() - 1]),
                                }
                            } else {
                                ChannelLine {
                                    source: source_,
                                    is_action: false,
                                    message: msg.clone(),
                                }
                            };

                            // FIXME: This needs to handle requests in /me
                            match check_command_in_channel(mynick, msg) {
                                Some(ref command) => {
                                    handle_bot_command(&server, command, target, Some(source))
                                }
                                None => {
                                    channel_data.add_line(line);
                                }
                            }
                        } else {
                            // FIXME: trailing \n
                            warn!("UNEXPECTED TARGET {} in message {}", target, message);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

// Take a message in the channel, and see if it was a message sent to
// this bot.
fn check_command_in_channel(mynick: &str, msg: &String) -> Option<String> {
    if !msg.starts_with(mynick) {
        return None;
    }
    let after_nick = &msg[mynick.len()..];
    if !after_nick.starts_with(":") && !after_nick.starts_with(",") {
        return None;
    }
    let after_punct = &after_nick[1..];
    Some(String::from(after_punct.trim_left()))
}

fn handle_bot_command(server: &IrcServer,
                      command: &str,
                      response_target: &str,
                      response_username: Option<&str>) {

    let send_line = |response_username: Option<&str>, line: &str| {
        let adjusted_line = match response_username {
            None => String::from(line),
            Some(username) => String::from(username) + ", " + line,
        };
        server
            .send_privmsg(response_target, &adjusted_line)
            .unwrap();
    };

    if command == "help" {
        send_line(response_username, "The commands I understand are:");
        send_line(None, "  help     Send this message.");
        return;
    }

    send_line(response_username,
              "Sorry, I don't understand that command.  Try 'help'.");
}

struct ChannelLine {
    source: String,
    is_action: bool,
    message: String,
}

struct TopicData {
    lines: Vec<ChannelLine>,
}

struct ChannelData {
    current_topic: Option<TopicData>,
}

impl TopicData {
    fn new() -> TopicData {
        TopicData { lines: vec![] }
    }
}

impl ChannelData {
    fn new() -> ChannelData {
        ChannelData { current_topic: None }
    }

    fn add_line(&mut self, line: ChannelLine) {
        if line.message.starts_with("Topic:") {
            self.start_topic();
        }
        if line.source == "trackbot" && line.is_action == true &&
           line.message == "is ending a teleconference." {
            self.end_topic();
        }
        print!("{} {} {}\n", line.source, line.is_action, line.message);
        match self.current_topic {
            None => (),
            Some(ref mut data) => {
                data.lines.push(line);
            }
        }
    }

    fn start_topic(&mut self) {
        if self.current_topic.is_some() {
            self.end_topic();
        }

        self.current_topic = Some(TopicData::new());
    }

    fn end_topic(&mut self) {
        // TODO: Test the topic boundary code.
        // FIXME: Do something with the data rather than throwing it away!
        self.current_topic = None;
    }
}
