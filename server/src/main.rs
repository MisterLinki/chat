use std::{thread, net::{TcpStream, TcpListener}, io::{Write, Read}, sync::{Mutex, Arc} };

use rusqlite::{Connection, Statement};
use chrono::Local;

struct Database { connection: Connection }

impl Database {
    fn new(name_account: &str) -> Self {
        let connection: Connection = Connection::open(format!("./{}.db", name_account)).unwrap();

        connection.execute(
            "
            CREATE TABLE IF NOT EXISTS Pseudo (
                pseudo TEXT PRIMARY KEY NOT NULL,
                mdp TEXT NOT NULL
            );
            ", 
        []
        ).unwrap();

        connection.execute(
            "
            CREATE TABLE IF NOT EXISTS Message (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                pseudo TEXT NOT NULL,
                message TEXT,
    
                FOREIGN KEY (pseudo) REFERENCES Pseudo(pseudo)
            );
            ", 
            []
        ).unwrap();

        Self { connection }
    }
    fn add(&self, account: &String, message: &str) {
        let resultat = &self.connection.execute(
            "
            INSERT INTO Message(date, pseudo, message) VALUES (?1, ?2, ?3);",
            [format!("{}", Local::now().format("%d/%m/%Y %H:%M")), account.to_string(), message.to_string()],
        );

        if resultat.is_err() {
            let _ = &self.connection.execute("INSERT INTO Pseudo VALUES (?1, ?2)", [account.to_string(), "moncul".to_string()]).unwrap();
            self.add(account, message);
        }
    }
    fn read(&mut self, name: &str, mut stream: &TcpStream ) {
        let mut stmt: Statement = self.connection.prepare("SELECT * FROM Message").unwrap();

        let rows = stmt.query_map([], |col| {
            Ok(( col.get::<_, i32>(0)?, col.get::<_, String>(1)?, col.get::<_, String>(2)?, col.get::<_, String>(3)?))
        }).unwrap();

        for col in rows {
            let (_id, date, mut pseudo, message) = col.unwrap();

            if pseudo == name { pseudo = "You".to_string(); }
            stream.write_all(format!("\n[{} : {}] : {}", date, pseudo.trim_end(), message).as_bytes()).unwrap();
        }
        
    }

    fn connection_database(stmt: &mut Statement, name_account: &str, mdp: &str) -> bool {
        let rows = stmt.query_map([], |col| {   Ok(( col.get::<_, String>(0)?, col.get::<_, String>(1)?)) }).unwrap();

        for col in rows {
            let (name, mdp_account) = col.unwrap();

            if name_account == name && mdp == mdp_account {
                return true;
            }
        }
        false
    }

    fn register_database(&self, stmt: &mut Statement, name_account: &str, mdp: &str) -> bool {
        let rows = stmt.query_map([], |col| {   Ok(( col.get::<_, String>(0)?, col.get::<_, String>(1)?)) }).unwrap();

        for col in rows {
            let (pseudo, _mdp) = col.unwrap();
            if pseudo == name_account {
                return false;
            }
        }

        let _ = &self.connection.execute("INSERT INTO Pseudo VALUES (?1, ?2)", [name_account.to_string(), mdp.to_string()]).unwrap();
        true
    }

    fn connection_or_register(&self, mut stream: &TcpStream) -> String {
        let mut stmt: Statement = self.connection.prepare("SELECT * FROM Pseudo").unwrap();

        loop {
            stream.write_all("write connection or register.".as_bytes()).unwrap();

            let mut message_buffer: [u8; 1024] = [0; 1024];
            let bytes: usize = match stream.read(&mut message_buffer){
                Ok(bytes) => bytes,
                Err(e) => return e.to_string(),
            };

            if String::from_utf8_lossy(&message_buffer[..bytes]).trim_end().to_string() == "connection".to_string(){
                stream.write_all("enter your name:".as_bytes()).unwrap();

                let mut message_buffer: [u8; 1024] = [0; 1024];
                let bytes: usize = match stream.read(&mut message_buffer){
                    Ok(bytes) => bytes,
                    Err(e) => return e.to_string(),
                };
                let name: String = String::from_utf8_lossy(&message_buffer[..bytes]).trim_end().to_string();
                
                stream.write_all("enter your password:".as_bytes()).unwrap();
                let mut message_buffer: [u8; 1024] = [0; 1024];
                let bytes: usize = match stream.read(&mut message_buffer){
                    Ok(bytes) => bytes,
                    Err(e) => return e.to_string(),
                };
                let mdp: String =  String::from_utf8_lossy(&message_buffer[..bytes]).trim_end().to_string();

                if Self::connection_database(&mut stmt, &name, mdp.as_str()){ 
                    stream.write_all("connection etablished".as_bytes()).unwrap();
                    return name; 
                } else { 
                    stream.write_all("connection incorrect".as_bytes()).unwrap()
                }
            }
            else if String::from_utf8_lossy(&message_buffer[..bytes]).trim_end().to_string() == "register".to_string() {
                stream.write_all("enter your name:".as_bytes()).unwrap();
                let mut message_buffer: [u8; 1024] = [0; 1024];
                let bytes: usize = match stream.read(&mut message_buffer){
                    Ok(bytes) => bytes,
                    Err(e) => return e.to_string(),
                };
                let name: String = String::from_utf8_lossy(&message_buffer[..bytes]).trim_end().to_string();
                
                stream.write_all("enter your password:".as_bytes()).unwrap();
                let mut message_buffer: [u8; 1024] = [0; 1024];
                let bytes: usize = match stream.read(&mut message_buffer){
                    Ok(bytes) => bytes,
                    Err(e) => return e.to_string(),
                };
                let mdp: String =  String::from_utf8_lossy(&message_buffer[..bytes]).trim_end().to_string();

                if Self::register_database(&self, &mut stmt, &name, mdp.as_str()){ 
                    stream.write_all("account registered".as_bytes()).unwrap();
                    return name; 
                } else { 
                    stream.write_all("account already registered".as_bytes()).unwrap()
                }
            }
        }
    }
}

struct Messagechat { message_buffer: [u8; 1024] }

impl Messagechat {
    fn new() -> Self {
        Self { message_buffer: [0; 1024] }
    }

    fn main_thread(&mut self, mut stream: TcpStream, database: Database, name_client: String, list_message: Arc<Mutex<Vec<(String, TcpStream)>>>) {     
        println!("{} connected", stream.peer_addr().unwrap());
        println!("{:?}", list_message);
        loop {
            self.message_buffer = [0; 1024];
            
            match stream.read(&mut self.message_buffer) {
                Ok(bytes) => {
                    match bytes {
                        0 => {
                            eprintln!("connection closed with {}", stream.peer_addr().unwrap());
                            break;
                        },
                        _ => {
                            let message: String = String::from_utf8_lossy(&self.message_buffer[..bytes]).to_string();
                            let message_spilted: std::str::Split<&str> = message.split("--");
                            match message_spilted.clone().count(){
                                2 => {
                                    let private_name: String = message_spilted.clone().nth(1).unwrap().to_string();
                                    let private_message: String = message_spilted.clone().nth(0).unwrap().to_string();

                                    match list_message.lock().unwrap().iter().find_map(|(name, client)| {
                                        if name == &private_name {
                                            Some(client)
                                        } else {
                                            None
                                        }
                                    }) {
                                        Some(mut private_client) => {                                    
                                            private_client.write_all(format!("private : [{} : {}] : {}",Local::now().format("%d/%m/%Y %H:%M"), name_client, private_message).as_bytes()).unwrap();
                                        },
                                        None => {
                                            println!("Private client not found.");
                                            stream.write_all("Private client not found.".as_bytes()).unwrap();
                                        }
                                    }
                                },
                                _ => {
                                    for (client, ip) in list_message.lock().unwrap().iter() {
                                        let mut ip: &TcpStream = ip;

                                        if client != &name_client{
                                            ip.write_all(format!("[{} : {}] : {}", Local::now().format("%d/%m/%Y %H:%M"), name_client, message).as_bytes()).unwrap();
                                        }
                                    }
                                    database.add(&name_client, message.as_str());
                                },
                            }
                        }
                    }
                },
                Err(_) => {
                    eprintln!("Error connection lost with {}", stream.peer_addr().unwrap());
                    break;
                },
            }
        }
        let mut guard = list_message.lock().unwrap();
        if let Some(index) = guard.iter().position(|(_, client_stream)| {
            format!("{:?}", client_stream.peer_addr().unwrap()).trim_end() == format!("{:?}", stream.peer_addr().unwrap()).trim_end()
        }) {
            guard.remove(index);
            return; 
        }
    }
}

fn main(){
    let list_connection: Arc<Mutex<Vec<(String, TcpStream)>>> = Arc::new(Mutex::new(vec![]));
    let listener: TcpListener = TcpListener::bind("0.0.0.0:4545").unwrap();

    for stream in listener.incoming() {
        let stream: TcpStream = stream.unwrap();

        let list_connection_cloned = list_connection.clone();

        thread::spawn(move || {
            let mut database: Database = Database::new("database");
            let mut messagechat: Messagechat = Messagechat::new();
            
            let name: String = database.connection_or_register(&stream);
            let name_cloned: String = name.clone();

            if name == "An existing connection was forcibly closed by the remote host. (os error 10054)".to_string(){ return; }
            
            list_connection_cloned.lock().unwrap().push((name_cloned, stream.try_clone().unwrap()));

            database.read(name.as_str(), &stream);
            messagechat.main_thread(stream, database, name, list_connection_cloned)
            
        });
    }
}