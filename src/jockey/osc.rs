use std::{
    collections::HashMap,
    convert::TryInto,
    net::UdpSocket,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
    time::Duration,
};

use rosc::{OscMessage, OscPacket, OscType};

#[derive(Debug, Clone)]
pub struct OscValue {
    pub value: f32,
    pub address: String,
}

#[derive(Debug)]
pub struct OscReceiver {
    socket: Option<UdpSocket>,
    values: Arc<Mutex<HashMap<String, f32>>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
    current_port: Option<u16>,
}

impl OscReceiver {
    pub fn new() -> Self {
        Self {
            socket: None,
            values: Arc::new(Mutex::new(HashMap::new())),
            thread_handle: None,
            running: Arc::new(AtomicBool::new(false)),
            current_port: None,
        }
    }

    pub fn start(&mut self, port: u16) -> Result<(), String> {
        // Don't restart if already running on the same port
        if self.current_port == Some(port) && self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        if self.socket.is_some() {
            self.stop();
        }

        let addr = format!("127.0.0.1:{}", port);
        let socket = UdpSocket::bind(&addr)
            .map_err(|e| format!("Failed to bind OSC socket to {}: {}", addr, e))?;

        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(|e| format!("Failed to set socket timeout: {}", e))?;

        let values = Arc::clone(&self.values);
        let running = Arc::clone(&self.running);
        let socket_clone = socket
            .try_clone()
            .map_err(|e| format!("Failed to clone socket: {}", e))?;

        running.store(true, Ordering::Relaxed);

        let handle = thread::spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];

            while running.load(Ordering::Relaxed) {
                match socket_clone.recv_from(&mut buf) {
                    Ok((size, _addr)) => {
                        if let Ok((_remaining, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                            Self::process_packet(&values, packet);
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock &&
                           e.kind() != std::io::ErrorKind::TimedOut {
                            log::warn!("OSC receive error: {}", e);
                            break;
                        }
                    }
                }
            }
            log::debug!("OSC receiver thread stopped");
        });

        self.socket = Some(socket);
        self.thread_handle = Some(handle);
        self.current_port = Some(port);

        log::info!("OSC receiver started on port {}", port);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(socket) = self.socket.take() {
            drop(socket);
        }

        if let Some(handle) = self.thread_handle.take() {
            if let Err(e) = handle.join() {
                log::warn!("Failed to join OSC receiver thread: {:?}", e);
            }
        }

        self.current_port = None;
        log::info!("OSC receiver stopped");
    }

    fn process_packet(values: &Arc<Mutex<HashMap<String, f32>>>, packet: OscPacket) {
        match packet {
            OscPacket::Message(msg) => {
                Self::process_message(values, msg);
            }
            OscPacket::Bundle(bundle) => {
                for packet in bundle.content {
                    Self::process_packet(values, packet);
                }
            }
        }
    }

    fn process_message(values: &Arc<Mutex<HashMap<String, f32>>>, msg: OscMessage) {
        if msg.args.is_empty() {
            return;
        }

        let value = match &msg.args[0] {
            OscType::Float(f) => *f,
            OscType::Double(d) => *d as f32,
            OscType::Int(i) => *i as f32,
            OscType::Long(l) => *l as f32,
            OscType::Bool(b) => if *b { 1.0 } else { 0.0 },
            _ => return,
        };

        if let Ok(mut values_map) = values.lock() {
            log::debug!("OSC received: {} = {}", msg.addr, value);
            values_map.insert(msg.addr, value);
        } else {
            log::warn!("Failed to lock OSC values map");
        }
    }

    pub fn get_value(&self, address: &str) -> Option<f32> {
        self.values.lock().ok()?.get(address).copied()
    }

    pub fn get_all_values(&self) -> HashMap<String, f32> {
        self.values.lock().map(|guard| guard.clone()).unwrap_or_default()
    }
}

impl Drop for OscReceiver {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Debug, Clone)]
pub struct OscConfig {
    pub port: u16,
    pub mappings: HashMap<String, String>,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            port: 9000,
            mappings: HashMap::new(),
        }
    }
}

impl OscConfig {
    pub fn from_yaml(value: &serde_yaml::Value) -> Result<Self, String> {
        let mut config = Self::default();

        if let Some(port) = value.get("port") {
            config.port = port.as_u64()
                .ok_or("OSC port must be a number")?
                .try_into()
                .map_err(|_| "OSC port must be between 0 and 65535")?;
        }

        if let Some(mappings) = value.get("mappings") {
            if let Some(mappings_obj) = mappings.as_mapping() {
                for (key, val) in mappings_obj {
                    let key_str = key.as_str()
                        .ok_or("OSC mapping key must be a string")?
                        .to_string();
                    let val_str = val.as_str()
                        .ok_or("OSC mapping value must be a string")?
                        .to_string();
                    config.mappings.insert(key_str, val_str);
                }
            }
        }

        Ok(config)
    }
}