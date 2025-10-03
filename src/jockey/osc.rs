use std::{
    collections::HashMap,
    convert::TryInto,
    net::UdpSocket,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
    time::Duration,
};

use rosc::{OscMessage, OscPacket, OscType};

#[derive(Debug, Clone, PartialEq)]
pub enum OscDataType {
    Float,
    Int,
    Bool,
}

impl Default for OscDataType {
    fn default() -> Self {
        Self::Float
    }
}

#[derive(Debug, Clone)]
pub struct OscMapping {
    pub address: String,
    pub data_type: OscDataType,
}

#[derive(Debug, Clone)]
pub struct OscValue {
    pub value: f32,
    pub address: String,
}

#[derive(Debug, Clone)]
pub enum OscUniformValue {
    Float(f32),
    Int(i32),
    Bool(bool),
}

#[derive(Debug)]
pub struct OscReceiver {
    socket: Option<UdpSocket>,
    values: Arc<Mutex<HashMap<String, OscUniformValue>>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
    current_port: Option<u16>,
    type_mappings: Arc<Mutex<HashMap<String, OscDataType>>>,
}

impl OscReceiver {
    pub fn new() -> Self {
        Self {
            socket: None,
            values: Arc::new(Mutex::new(HashMap::new())),
            thread_handle: None,
            running: Arc::new(AtomicBool::new(false)),
            current_port: None,
            type_mappings: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update_type_mappings(&self, config: &OscConfig) {
        if let Ok(mut mappings) = self.type_mappings.lock() {
            mappings.clear();
            for (_, mapping) in &config.mappings {
                mappings.insert(mapping.address.clone(), mapping.data_type.clone());
            }
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
        let type_mappings = Arc::clone(&self.type_mappings);
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
                            Self::process_packet(&values, &type_mappings, packet);
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

    fn process_packet(
        values: &Arc<Mutex<HashMap<String, OscUniformValue>>>,
        type_mappings: &Arc<Mutex<HashMap<String, OscDataType>>>,
        packet: OscPacket,
    ) {
        match packet {
            OscPacket::Message(msg) => {
                Self::process_message(values, type_mappings, msg);
            }
            OscPacket::Bundle(bundle) => {
                for packet in bundle.content {
                    Self::process_packet(values, type_mappings, packet);
                }
            }
        }
    }

    fn process_message(
        values: &Arc<Mutex<HashMap<String, OscUniformValue>>>,
        type_mappings: &Arc<Mutex<HashMap<String, OscDataType>>>,
        msg: OscMessage,
    ) {
        if msg.args.is_empty() {
            return;
        }

        // Get the expected data type for this address
        let expected_type = type_mappings
            .lock()
            .ok()
            .and_then(|mappings| mappings.get(&msg.addr).cloned())
            .unwrap_or(OscDataType::Float); // Default to Float

        // Convert the OSC value based on the expected type
        let value = match Self::convert_osc_value(&msg.args[0], &expected_type) {
            Some(v) => v,
            None => {
                log::warn!("Failed to convert OSC value at {} to {:?}", msg.addr, expected_type);
                return;
            }
        };

        if let Ok(mut values_map) = values.lock() {
            log::debug!("OSC received: {} = {:?} (as {:?})", msg.addr, value, expected_type);
            values_map.insert(msg.addr, value);
        } else {
            log::warn!("Failed to lock OSC values map");
        }
    }

    fn convert_osc_value(osc_arg: &OscType, target_type: &OscDataType) -> Option<OscUniformValue> {
        match target_type {
            OscDataType::Float => match osc_arg {
                OscType::Float(f) => Some(OscUniformValue::Float(*f)),
                OscType::Double(d) => Some(OscUniformValue::Float(*d as f32)),
                OscType::Int(i) => Some(OscUniformValue::Float(*i as f32)),
                OscType::Long(l) => Some(OscUniformValue::Float(*l as f32)),
                OscType::Bool(b) => Some(OscUniformValue::Float(if *b { 1.0 } else { 0.0 })),
                _ => None,
            },
            OscDataType::Int => match osc_arg {
                OscType::Int(i) => Some(OscUniformValue::Int(*i)),
                OscType::Long(l) => Some(OscUniformValue::Int(*l as i32)),
                OscType::Float(f) => Some(OscUniformValue::Int(f.round() as i32)),
                OscType::Double(d) => Some(OscUniformValue::Int(d.round() as i32)),
                OscType::Bool(b) => Some(OscUniformValue::Int(if *b { 1 } else { 0 })),
                _ => None,
            },
            OscDataType::Bool => match osc_arg {
                OscType::Bool(b) => Some(OscUniformValue::Bool(*b)),
                OscType::Int(i) => Some(OscUniformValue::Bool(*i != 0)),
                OscType::Long(l) => Some(OscUniformValue::Bool(*l != 0)),
                OscType::Float(f) => Some(OscUniformValue::Bool(*f != 0.0)),
                OscType::Double(d) => Some(OscUniformValue::Bool(*d != 0.0)),
                _ => None,
            },
        }
    }

    pub fn get_value(&self, address: &str) -> Option<OscUniformValue> {
        self.values.lock().ok()?.get(address).cloned()
    }

    pub fn get_all_values(&self) -> HashMap<String, OscUniformValue> {
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
    pub mappings: HashMap<String, OscMapping>,
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

                    let mapping = match val {
                        // Simple string format: "uniform_name": "/osc/address"
                        serde_yaml::Value::String(address) => {
                            OscMapping {
                                address: address.clone(),
                                data_type: OscDataType::default(), // Float
                            }
                        },
                        // Extended format: "uniform_name": { "address": "/osc/address", "type": "float" }
                        serde_yaml::Value::Mapping(map) => {
                            let address = map.get(&serde_yaml::Value::String("address".to_string()))
                                .and_then(|v| v.as_str())
                                .ok_or("OSC mapping must have 'address' field")?
                                .to_string();

                            let data_type = match map.get(&serde_yaml::Value::String("type".to_string()))
                                .and_then(|v| v.as_str()) {
                                Some("float") => OscDataType::Float,
                                Some("int") => OscDataType::Int,
                                Some("bool") => OscDataType::Bool,
                                Some(other) => return Err(format!("Unknown OSC data type: {}", other)),
                                None => OscDataType::default(), // Float
                            };

                            OscMapping { address, data_type }
                        },
                        _ => return Err("OSC mapping value must be a string or object".to_string()),
                    };

                    config.mappings.insert(key_str, mapping);
                }
            }
        }

        Ok(config)
    }
}