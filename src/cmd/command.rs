use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::{resp::RespDT, store::cache::Db};

const SET_CMD_RESP: &'static str = "OK";
const PONG_CMD_RESP: &'static str = "PONG";

trait CommandRespond {
    async fn response_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}
#[derive(Debug)]
pub struct PingCommand;

#[derive(Debug)]
pub struct EchoCommand {
    pub message: String,
}

#[derive(Debug)]
pub struct SetCommand {
    pub key: String,
    pub value: String,
    pub expiry: Option<SystemTime>,
    pub cache: Arc<Db>,
}

#[derive(Debug)]
pub struct GetCommand {
    pub key: String,
    pub cache: Arc<Db>,
}

impl CommandRespond for PingCommand {
    async fn response_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(RespDT::SimpleString(PONG_CMD_RESP.to_string()).encode_raw())
    }
}

impl CommandRespond for EchoCommand {
    async fn response_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(RespDT::SimpleString(self.message.clone()).encode_raw())
    }
}

impl CommandRespond for SetCommand {
    async fn response_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.cache
            .store(self.key.clone(), self.value.clone(), self.expiry)
            .await;
        Ok(RespDT::SimpleString(SET_CMD_RESP.to_string()).encode_raw())
    }
}

impl CommandRespond for GetCommand {
    async fn response_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.cache.fetch(self.key.clone()).await {
            Some(val) => Ok(RespDT::SimpleString(val).encode_raw()),
            None => Ok(RespDT::Null.encode_raw()),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping(PingCommand),
    Echo(EchoCommand),
    Set(SetCommand),
    Get(GetCommand),
}

impl Command {
    pub async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self {
            Command::Ping(cmd) => cmd.response_bytes().await,
            Command::Echo(cmd) => cmd.response_bytes().await,
            Command::Set(cmd) => cmd.response_bytes().await,
            Command::Get(cmd) => cmd.response_bytes().await,
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    UnknownCommand,
    InvalidCommand,
    InvalidArguments,
}

pub struct RespCache {
    pub cache: Arc<Db>,
    pub resp: RespDT,
}

impl RespCache {
    pub fn new(cache: Arc<Db>, resp: RespDT) -> Self {
        RespCache { cache, resp }
    }
}

impl TryFrom<RespCache> for Command {
    type Error = CommandError;

    fn try_from(value: RespCache) -> Result<Self, Self::Error> {
        let (cmd, args) = value.resp.extract_array().unwrap();
        match cmd.as_str() {
            "ping" => Ok(Command::Ping(PingCommand)),
            "echo" => {
                if args.len() != 1 {
                    return Err(CommandError::InvalidArguments);
                }
                match args.first().unwrap().extract_bulk_str() {
                    Ok(message) => Ok(Command::Echo(EchoCommand { message })),
                    Err(_) => Err(CommandError::InvalidCommand),
                }
            }
            "set" => {
                if args.len() < 2 {
                    Err(CommandError::InvalidArguments)
                } else {
                    if args.len() == 4
                        && args
                            .get(2)
                            .unwrap()
                            .extract_bulk_str()
                            .unwrap()
                            .to_lowercase()
                            .eq("px")
                    {
                        let expiry = args
                            .get(3)
                            .unwrap()
                            .extract_bulk_str()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap();
                        let expiry = SystemTime::now()
                            .checked_add(Duration::from_millis(expiry))
                            .unwrap();
                        return Ok(Command::Set(SetCommand {
                            key: args.first().unwrap().extract_bulk_str().unwrap(),
                            value: args.get(1).unwrap().extract_bulk_str().unwrap(),
                            expiry: Some(expiry),
                            cache: value.cache,
                        }));
                    }
                    Ok(Command::Set(SetCommand {
                        key: args.first().unwrap().extract_bulk_str().unwrap(),
                        value: args.get(1).unwrap().extract_bulk_str().unwrap(),
                        expiry: None,
                        cache: value.cache,
                    }))
                }
            }
            "get" => {
                if args.len() == 1 {
                    Ok(Command::Get(GetCommand {
                        key: args.first().unwrap().extract_bulk_str().unwrap(),
                        cache: value.cache,
                    }))
                } else {
                    Err(CommandError::InvalidArguments)
                }
            }
            _ => Err(CommandError::UnknownCommand),
        }
    }
}
