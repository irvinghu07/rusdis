use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::resp::RespDT;

type Cache = Arc<Mutex<HashMap<String, String>>>;

#[derive(Debug)]
pub enum Command {
    Ping(RespDT),
    Echo(RespDT),
    Set(String, String),
    Get(String),
}

impl Command {
    pub async fn execute(&self, db: Cache) -> Vec<u8> {
        match self {
            Command::Ping(resp) => Self::handle_ping(resp),
            Command::Echo(resp) => Self::handle_echo(resp),
            Command::Set(key, val) => Self::handle_set(db, key.to_string(), val.to_string()).await,
            Command::Get(key) => Self::handle_get(db, key).await,
        }
    }

    fn handle_ping(resp: &RespDT) -> Vec<u8> {
        resp.encode_raw()
    }

    fn handle_echo(resp: &RespDT) -> Vec<u8> {
        resp.encode_raw()
    }

    async fn handle_set(db: Cache, key: String, val: String) -> Vec<u8> {
        db.lock().await.insert(key, val);
        RespDT::SimpleString("OK".to_string()).encode_raw()
    }

    async fn handle_get(db: Cache, key: &String) -> Vec<u8> {
        match db.lock().await.get(key) {
            Some(val) => RespDT::SimpleString(val.to_string()).encode_raw(),
            None => RespDT::Null.encode_raw(),
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    UnknownCommand,
    InvalidCommand,
    InvalidArguments,
}

impl TryFrom<RespDT> for Command {
    type Error = CommandError;

    fn try_from(value: RespDT) -> Result<Self, Self::Error> {
        let (cmd, args) = value.extract_resp().unwrap();
        match cmd.as_str() {
            "ping" => Ok(Command::Ping(RespDT::SimpleString("PONG".to_string()))),
            "echo" => {
                if args.len() != 1 {
                    return Err(CommandError::InvalidArguments);
                }
                match args.first().unwrap().extract_bulk_str() {
                    Ok(arg) => Ok(Command::Echo(RespDT::SimpleString(arg))),
                    Err(_) => Err(CommandError::InvalidCommand),
                }
            }
            "set" => {
                if args.len() < 2 {
                    Err(CommandError::InvalidArguments)
                } else {
                    Ok(Command::Set(
                        args.first().unwrap().extract_bulk_str().unwrap(),
                        args.last().unwrap().extract_bulk_str().unwrap(),
                    ))
                }
            }
            "get" => {
                if args.len() == 1 {
                    Ok(Command::Get(
                        args.first().unwrap().extract_bulk_str().unwrap(),
                    ))
                } else {
                    Err(CommandError::InvalidArguments)
                }
            }
            _ => Err(CommandError::UnknownCommand),
        }
    }
}
