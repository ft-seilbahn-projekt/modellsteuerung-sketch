use std::sync::Arc;
use std::thread;
use std::thread::Thread;
use std::time::Duration;
use serial2::SerialPort;
use serde::{Serialize, Deserialize};
use crate::{STATE};


#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Type {
    JST,
    RS485,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub ssid: String,
    pub password: String,
    pub rgb_led_num: u8,
    pub create_swarm: bool,
    pub swarm_name: String,
    pub swarm_pin: String,
    pub hostname: String,
    pub swarm_type: Type,
    pub input_ports: Vec<String>,
    pub output_ports: Vec<String>,
    pub led_ports: Vec<String>,
    pub servo_port: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ssid: "abab".to_string(),
            password: "cdcd".to_string(),
            rgb_led_num: 2,
            create_swarm: true,
            swarm_name: "helloworld".to_string(),
            swarm_pin: "1234".to_string(),
            hostname: "kelda".to_string(),
            swarm_type: Type::JST,
            input_ports: vec![],
            output_ports: vec![],
            led_ports: vec![],
            servo_port: "".to_string(),
        }
    }
}

pub enum Command {
    Connect,
    Disconnect,
    Apply(Settings),
    Send(String),
}

fn reset_board(serial: &mut SerialPort) {
    serial.write(b"res\r\n").unwrap();
    // Consume until >>> is received
    let mut buffer = [0; 256];
    let mut read = false;
    let mut full = String::new();
    while !read {
        if let Ok(r) = serial.read(&mut buffer) {
            let data = String::from_utf8_lossy(&buffer[0..r]);
            full.push_str(&data);

            if full.contains(">>>") {
                read = true;
            }
        }
    }
}

pub(crate) fn serial_thread() {
    let mut serial: Option<SerialPort> = None;
    let mut buffer = [0; 256];

    loop {
        let mut state = STATE.lock().unwrap();

        if !state.connected {
            if let Ok(paths) = SerialPort::available_ports() {
                state.port_list = paths
                    .iter()
                    .map(|x| x.to_str().unwrap().to_string())
                    .collect();
            }
        }

        if let Some(command) = state.command_queue.pop() {
            match command {
                Command::Connect => {
                    if state.port == "" {
                        state.console_log_lines.push("* no port selected".to_string());
                        continue;
                    }

                    let port = state.port.clone();

                    if let Ok(serial_port) = SerialPort::open(&port, 115200) {
                        serial = Some(serial_port);
                    } else {
                        state.console_log_lines.push(format!("* failed to connect to {}", port));
                        continue;
                    }
                    state.console_log_lines.push(format!("* connected to {}", port));

                    state.connected = true;
                }
                Command::Disconnect => {
                    let port = state.port.clone();
                    state.console_log_lines.push(format!("* disconnected from {}", port));
                    thread::sleep(Duration::from_millis(1000));
                    state.connected = false;
                }
                Command::Apply(settings) => {
                    state.console_log_lines.push("* resetting. to force, click key now".to_string());
                    drop(state);
                    thread::sleep(Duration::from_millis(500));
                    reset_board(serial.as_mut().unwrap());
                    state = STATE.lock().unwrap();
                    state.console_log_lines.push("* reset successful".to_string());
                    let serial = serial.as_mut().unwrap();
                    serial.write(b"stp\r\n").unwrap(); // Open settings prompt
                    thread::sleep(Duration::from_millis(100));

                    // go to local settings and wait for "AP-Mode:  "
                    serial.write(b"1\r\n").unwrap();

                    let mut data;
                    let mut local_buffer = [0; 256];

                    drop(state);
                    loop {
                        if let Ok(read) = serial.read(&mut local_buffer) {
                            let line = String::from_utf8_lossy(&local_buffer[0..read]);
                            if line.contains("AP-Mode:  ") {
                                data = line.to_string();
                                break;
                            }
                        }
                    }
                    state = STATE.lock().unwrap();

                    // Get current setting for AP-Mode
                    let ap_mode = data.split("AP-Mode:  ").collect::<Vec<&str>>()[1]
                        .split("\n").collect::<Vec<&str>>()[0].trim();


                    if ap_mode.to_lowercase().contains("ap") {
                        // AP-Mode is enabled, disable it
                        serial.write(b"1\n").unwrap();
                        state.console_log_lines.push("* disabled AP-Mode".to_string());
                    }
                    thread::sleep(Duration::from_millis(100));

                    serial.write(b"2\n").unwrap();
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    serial.write(settings.ssid.as_bytes()).unwrap();
                    serial.write(b"\n").unwrap();
                    state.console_log_lines.push("* set SSID".to_string());
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    serial.write(b"3\n").unwrap();
                    state.console_log_lines.push("* set password".to_string());
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    serial.write(settings.password.as_bytes()).unwrap();
                    serial.write(b"\n").unwrap();
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    serial.write(b"5\n").unwrap();
                    state.console_log_lines.push("* set led num".to_string());
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    serial.write(settings.rgb_led_num.to_string().as_bytes()).unwrap();
                    serial.write(b"\n").unwrap();
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    // go out of local settings
                    serial.write(b"0\n").unwrap();
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    serial.write(b"y\n").unwrap(); // Save settings

                    let mut buffer = [0; 256];
                    loop {
                        if let Ok(r) = serial.read(&mut buffer) {
                            let data = String::from_utf8_lossy(&buffer[0..r]);
                            if data.contains(">>>") {
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    serial.write(b"stp\n").unwrap(); // Open settings prompt
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    // go to swarm settings
                    serial.write(b"2\n").unwrap();
                    drop(state);
                    thread::sleep(Duration::from_millis(100));

                    // read until "connected to swarm"
                    let mut data;
                    let mut local_buffer = [0; 256];
                    loop {
                        if let Ok(read) = serial.read(&mut local_buffer) {
                            let line = String::from_utf8_lossy(&local_buffer[0..read]);
                            if line.contains("This device is connected to swarm \"") {
                                data = line.to_string();
                                break;
                            }
                        }
                    }
                    state = STATE.lock().unwrap();

                    // Get current setting for swarm name
                    let swarm_name = data.split("connected to swarm \"").collect::<Vec<&str>>()[1]
                        .split("\"").collect::<Vec<&str>>()[0].trim();

                    if swarm_name != settings.swarm_name {
                        if settings.create_swarm {
                            serial.write(b"1\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(200));
                            state = STATE.lock().unwrap();
                            serial.write(settings.swarm_name.as_bytes()).unwrap();
                            thread::sleep(Duration::from_millis(200));
                            serial.write(b"\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(500));
                            state = STATE.lock().unwrap();
                            serial.write(settings.swarm_pin.as_bytes()).unwrap();
                            serial.write(b"\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(200));
                            state = STATE.lock().unwrap();

                            serial.write(b"y\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(1000));
                            state = STATE.lock().unwrap();

                            state.console_log_lines.push("* created swarm".to_string());
                        } else {
                            serial.write(b"1\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(100));
                            state = STATE.lock().unwrap();
                            serial.write(settings.swarm_name.as_bytes()).unwrap();
                            serial.write(b"\n\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(200));
                            state = STATE.lock().unwrap();
                            serial.write(settings.swarm_pin.as_bytes()).unwrap();
                            thread::sleep(Duration::from_millis(200));
                            serial.write(b"\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(100));
                            state = STATE.lock().unwrap();

                            serial.write(b"y\n").unwrap();
                            drop(state);
                            thread::sleep(Duration::from_millis(1000));
                            state = STATE.lock().unwrap();

                            state.console_log_lines.push("* joined swarm".to_string());
                        }
                    } else {
                        state.console_log_lines.push("* already in swarm".to_string());
                    }

                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();
                    serial.write(b"0\n").unwrap(); // close swarm settings

                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();
                    serial.write(b"3\n").unwrap(); // open alias settings

                    let mut alias = 1;
                    // set hostname
                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();
                    serial.write(alias.to_string().as_bytes()).unwrap();
                    serial.write(b"\n").unwrap(); // select alias

                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();
                    serial.write(settings.hostname.as_bytes()).unwrap();
                    serial.write(b"\n").unwrap(); // set hostname
                    alias += 1;

                    let analog_ports = if settings.swarm_type == Type::RS485 {
                        6
                    } else {
                        4
                    };

                    for i in 0..analog_ports {
                        drop(state);
                        thread::sleep(Duration::from_millis(500));
                        state = STATE.lock().unwrap();
                        serial.write(alias.to_string().as_bytes()).unwrap();
                        serial.write(b"\n").unwrap(); // select alias

                        drop(state);
                        thread::sleep(Duration::from_millis(100));
                        state = STATE.lock().unwrap();

                        if let Some(port) = settings.input_ports.get(i) {
                            serial.write(port.as_bytes()).unwrap();
                        }

                        serial.write(b"\n").unwrap(); // set value
                        alias += 1;

                        state.console_log_lines.push(format!("* set alias A{}", i));
                    }

                    for i in 0..2 {
                        drop(state);
                        thread::sleep(Duration::from_millis(500));
                        state = STATE.lock().unwrap();
                        serial.write(alias.to_string().as_bytes()).unwrap();
                        serial.write(b"\n").unwrap(); // select alias

                        drop(state);
                        thread::sleep(Duration::from_millis(100));
                        state = STATE.lock().unwrap();

                        if let Some(port) = settings.output_ports.get(i) {
                            serial.write(port.as_bytes()).unwrap();
                        }

                        serial.write(b"\n").unwrap(); // set value
                        alias += 1;

                        state.console_log_lines.push(format!("* set alias M{}", i));
                    }

                    for i in 0..settings.rgb_led_num {
                        drop(state);
                        thread::sleep(Duration::from_millis(500));
                        state = STATE.lock().unwrap();
                        serial.write(alias.to_string().as_bytes()).unwrap();
                        serial.write(b"\n").unwrap(); // select alias

                        drop(state);
                        thread::sleep(Duration::from_millis(100));
                        state = STATE.lock().unwrap();

                        if let Some(port) = settings.led_ports.get(i as usize) {
                            serial.write(port.as_bytes()).unwrap();
                        }

                        serial.write(b"\n").unwrap(); // set value
                        alias += 1;

                        state.console_log_lines.push(format!("* set alias LED{}", i));
                    }

                    drop(state);
                    thread::sleep(Duration::from_millis(500));
                    state = STATE.lock().unwrap();
                    serial.write(alias.to_string().as_bytes()).unwrap();
                    serial.write(b"\n").unwrap(); // select alias

                    drop(state);
                    thread::sleep(Duration::from_millis(100));
                    state = STATE.lock().unwrap();

                    if !settings.servo_port.is_empty() {
                        serial.write(settings.servo_port.as_bytes()).unwrap();
                    }

                    serial.write(b"\n").unwrap(); // set value

                    state.console_log_lines.push(format!("* set alias SERVO"));


                    drop(state);
                    thread::sleep(Duration::from_millis(500));
                    state = STATE.lock().unwrap();
                    serial.write(b"0\n").unwrap(); // close alias settings

                    drop(state);
                    thread::sleep(Duration::from_millis(500));
                    state = STATE.lock().unwrap();
                    serial.write(b"y\n").unwrap(); // save settings

                    drop(state);
                    thread::sleep(Duration::from_millis(500));
                    state = STATE.lock().unwrap();
                    serial.write(b"0\n").unwrap(); // Close settings prompt

                    // read all available data
                    let mut buffer = [0; 128];

                    loop {
                        if let Ok(read) = serial.read(&mut buffer) {
                            if read == 0 {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    state.console_log_lines.push("* done!".to_string());
                }
                Command::Send(data) => {
                    if let Some(serial) = &serial {
                        serial.write(data.as_bytes()).unwrap();
                        serial.write(b"\n").unwrap();
                        state.console_log_lines.push(format!("< {}", data));
                    } else {
                        state.console_log_lines.push("* Not connected!".to_string());
                    }
                }
            }
        }

        if state.connected {
            if let Some(serial) = &serial {
                if let Ok(read) = serial.read(&mut buffer) {
                    let data = String::from_utf8_lossy(&buffer[0..read]);

                    let mut lines = data.split("\n");
                    for line in lines {
                        if line.len() == 0 {
                            state.console_log_lines.push(line.to_string());
                            continue;
                        }

                        // check if last line has a newline
                        if let Some(last_line) = state.console_log_lines.last_mut() {
                            if !last_line.ends_with("\r") && last_line.starts_with("> ") {
                                last_line.push_str(line);
                                continue;
                            } else {
                                state.console_log_lines.push(format!("> {}", line.to_string()));
                            }
                        }
                    }
                }
            }
        }

        drop(state);

        thread::sleep(Duration::from_millis(100));
    }
}