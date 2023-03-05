mod support;
mod serial;

use std::borrow::Cow;
use std::io::Write;
use std::sync::{Mutex};
use std::thread;
use imgui::*;
use lazy_static::lazy_static;
use crate::serial::{Command, Settings, Type};

#[derive(Default)]
struct State {
    console_log_lines: Vec<String>,
    connected: bool,
    command_queue: Vec<Command>,
    port: String,
    port_list: Vec<String>,
    current_port: usize,
    settings: Settings,
    should_apply: bool,
}

// implement send
unsafe impl Send for State {}

lazy_static!(static ref STATE: Mutex<State> = Mutex::new(State::default()););

fn main() {
    // check for a config directory in the user's home directory
    // if it doesn't exist, create it
    let config_dir = dirs::config_dir().unwrap().join("swarm");
    if !config_dir.exists() {
        std::fs::create_dir(&config_dir).unwrap();
    }

    let system = support::init("swarm configurator");
    let mut command = String::new();
    let mut new_preset_name = String::new();

    let mut ssid: String = String::new();
    let mut password: String = String::new();
    let mut rgb_led_num: i32 = 0;
    let mut create_swarm: bool = false;
    let mut swarm_name: String = String::new();
    let mut swarm_pin: String = String::new();
    let mut hostname: String = String::new();
    let mut swarm_type: i32 = 0;
    let mut input_list: Vec<String> = vec![];
    let mut output_list: Vec<String> = vec![];
    let mut led_list: Vec<String> = vec![];
    let mut servo_port: String = String::new();
    let mut current_preset = 0;

    thread::spawn(move || {
        serial::serial_thread();
    });

    system.main_loop(move |_, ui| {
        let tile_width = ui.io().display_size[0] / 2.0 - 75.0;

        ui.window("controls")
            .size([tile_width, 200.0], Condition::Always)
            .position([50.0, 50.0], Condition::Always)
            .flags(WindowFlags::NO_RESIZE | WindowFlags::NO_MOVE | WindowFlags::NO_COLLAPSE)

            .build(|| {
                let mut state = STATE.lock().unwrap();

                let _d = ui.begin_enabled(!state.connected);

                // Drop down menu for selecting serial port
                let mut port_list = state.port_list.clone();
                port_list.insert(0, "Select a port".to_string());
                let port_list = port_list.iter().map(|x| x.as_str()).collect::<Vec<_>>();
                ui.combo(" ", &mut state.current_port, &port_list, |x| Cow::Owned(x.to_string()));

                if state.current_port != 0 && state.current_port <= state.port_list.len() {
                    state.port = state.port_list[state.current_port - 1].clone();
                }

                drop(_d);

                let _d = ui.begin_enabled(state.command_queue.len() == 0);

                if (!state.connected) && ui.button("connect") {
                    state.command_queue.push(Command::Connect);
                }

                if state.connected && ui.button("disconnect") {
                    state.command_queue.push(Command::Disconnect);
                }

                drop(_d);
                let _d = ui.begin_enabled(state.connected);

                // input field for a command
                ui.input_text("command", &mut command).build();

                if ui.button("send") {
                    state.command_queue.push(Command::Send(command.clone()));
                    command.clear();
                }

                drop(state);
            });

        ui.window("swarm configurator")
            .size([tile_width, ui.io().display_size[1] / 2.0 - 175.0], Condition::Always)
            .position([50.0, 300.0], Condition::Always)
            .flags(WindowFlags::NO_RESIZE | WindowFlags::NO_MOVE | WindowFlags::NO_COLLAPSE)
            .build(|| {
                let mut state = STATE.lock().unwrap();

                if state.should_apply {
                    ssid = state.settings.ssid.clone();
                    password = state.settings.password.clone();
                    rgb_led_num = state.settings.rgb_led_num as i32;
                    create_swarm = state.settings.create_swarm;
                    swarm_name = state.settings.swarm_name.clone();
                    swarm_pin = state.settings.swarm_pin.clone();
                    hostname = state.settings.hostname.clone();
                    swarm_type = if state.settings.swarm_type == Type::JST { 0 } else { 1 };
                    input_list = state.settings.input_ports.clone();
                    output_list = state.settings.output_ports.clone();
                    led_list = state.settings.led_ports.clone();
                    servo_port = state.settings.servo_port.clone();

                    state.should_apply = false;
                }

                ui.input_text("ssid", &mut ssid).build();
                ui.input_text("password", &mut password).build();
                ui.input_int("rgb led num", &mut rgb_led_num).build();
                ui.checkbox("create swarm", &mut create_swarm);
                ui.input_text("swarm name", &mut swarm_name).build();
                ui.input_text("swarm pin", &mut swarm_pin).build();
                ui.input_text("hostname", &mut hostname).build();
                ui.input_int("swarm type (0 for JST, 1 for RS485)", &mut swarm_type).build();

                ui.separator();
                ui.text("aliases:");

                let input_list_len_should_be = (4 + swarm_type * 2) as usize;

                if input_list.len() > input_list_len_should_be {
                    input_list.truncate(input_list_len_should_be);
                } else if input_list.len() < input_list_len_should_be {
                    for _ in input_list.len()..input_list_len_should_be {
                        input_list.push(String::new());
                    }
                }

                let mut i = 1;
                for mut input in input_list.iter_mut() {
                    let name = format!("A{}", i);
                    let name: &'static str = Box::leak(name.into_boxed_str());
                    ui.input_text(name, input).build();
                    i += 1;
                }

                if output_list.len() > 2 {
                    output_list.truncate(2);
                } else if output_list.len() < 2 {
                    for _ in output_list.len()..2 {
                        output_list.push(String::new());
                    }
                }

                let mut i = 1;
                for mut output in output_list.iter_mut() {
                    let name = format!("M{}", i);
                    let name: &'static str = Box::leak(name.into_boxed_str());
                    ui.input_text(name, output).build();
                    i += 1;
                }

                let rgb_list_len_should_be = rgb_led_num as usize;

                if led_list.len() > rgb_list_len_should_be {
                    led_list.truncate(rgb_list_len_should_be);
                } else if led_list.len() < rgb_list_len_should_be {
                    for _ in led_list.len()..rgb_list_len_should_be {
                        led_list.push(String::new());
                    }
                }

                let mut i = 1;
                for mut led in led_list.iter_mut() {
                    let name = format!("LED{}", i);
                    let name: &'static str = Box::leak(name.into_boxed_str());
                    ui.input_text(name, led).build();
                    i += 1;
                }

                ui.input_text("SERVO", &mut servo_port).build();

                let settings = Settings {
                    ssid: ssid.clone(),
                    password: password.clone(),
                    rgb_led_num: rgb_led_num.clone() as u8,
                    create_swarm: create_swarm.clone(),
                    swarm_name: swarm_name.clone(),
                    swarm_pin: swarm_pin.clone(),
                    hostname: hostname.clone(),
                    swarm_type: if swarm_type == 0 { Type::JST } else { Type::RS485 },
                    input_ports: input_list.clone(),
                    output_ports: output_list.clone(),
                    led_ports: led_list.clone(),
                    servo_port: servo_port.clone(),
                };

                state.settings = settings.clone();

                if ui.button("apply") {
                    state.command_queue.push(Command::Apply(settings));
                }
                drop(state);
            });

        ui.window("presets")
            .size([tile_width, ui.io().display_size[1] / 2.0 - 225.0], Condition::Always)
            .position([50.0, ui.io().display_size[1] / 2.0 + 175.0], Condition::Always)
            .flags(WindowFlags::NO_RESIZE | WindowFlags::NO_MOVE | WindowFlags::NO_COLLAPSE)
            .build(|| {
                // Get all the files in the config directory
                let mut state = STATE.lock().unwrap();
                let mut presets = std::fs::read_dir(&config_dir).unwrap();
                let mut preset_list = Vec::new();
                while let Some(Ok(preset)) = presets.next() {
                    if preset.file_type().unwrap().is_file() {
                        preset_list.push(preset.file_name().into_string().unwrap());
                    }
                }

                let mut preset_list = preset_list.iter().map(|x| x.as_str()).collect::<Vec<_>>();
                preset_list.insert(0, "Select a preset");

                ui.combo(" ", &mut current_preset, &preset_list, |x| Cow::Owned(x.to_string()));

                let mut preset_name = String::new();
                if current_preset != 0 && current_preset < preset_list.len() {
                    preset_name = preset_list[current_preset].to_string();
                }

                if ui.button("load") {
                    if let Ok(file) = std::fs::File::open(config_dir.join(&preset_name)) {
                        let reader = std::io::BufReader::new(file);
                        let settings: Settings = serde_json::from_reader(reader).unwrap();
                        state.settings = settings.clone();
                        state.should_apply = true;
                    }
                }

                if ui.button("save") {
                    if let Ok(str) = serde_json::to_string(&state.settings) {
                        let mut file = std::fs::File::create(config_dir.join(&preset_name)).unwrap();
                        file.write_all(str.as_bytes()).unwrap();
                    }
                }

                if ui.button("delete") {
                    std::fs::remove_file(config_dir.join(&preset_name)).unwrap();
                }

                ui.separator();

                ui.input_text("new preset name", &mut new_preset_name).build();

                if ui.button("new") {

                    if let Ok(str) = serde_json::to_string(&state.settings) {
                        let mut file = std::fs::File::create(config_dir.join(&new_preset_name)).unwrap();
                        file.write_all(str.as_bytes()).unwrap();
                        new_preset_name = String::new();
                    }
                }

                drop(state);
            });

        ui.window("console log")
            .size([ui.io().display_size[0] / 2.0 - 75.0, ui.io().display_size[1] - 100.0], Condition::Always)
            .position([ui.io().display_size[0] / 2.0 + 25.0, 50.0], Condition::Always)
            .flags(WindowFlags::NO_RESIZE | WindowFlags::NO_MOVE | WindowFlags::NO_COLLAPSE)
            .build(|| {
                let mut state = STATE.lock().unwrap();
                for x in state.console_log_lines.clone().iter().rev().into_iter().take(80) {
                    ui.text(x);
                }
                drop(state);
            });
    });
}
