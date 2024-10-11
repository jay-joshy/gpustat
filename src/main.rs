use chrono::prelude::*;
use clap::Parser;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use nix::unistd::{Uid, User};
use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, enums::device::UsedGpuMemory, Nvml};
use std::io::{self, Write};
use sysinfo::{Pid, PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum StatusError {
    #[error("Failed to parse hostname: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to convert string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Failed to load nvml library: {0}")]
    NvmlError(#[from] nvml_wrapper::error::NvmlError),
    #[error("Failed to call nix call: {0}")]
    NixError(#[from] nix::Error),
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Opts {
    #[arg(short = 'i', long, help = "Continually display every 2 seconds")]
    continuous: bool,
}

macro_rules! bold_limit {
    ($value:ident, $limit:expr, $color:expr, $($arg:tt)*) => {{
        let cell = Cell::new(format!($($arg)*)).fg($color);
        if $value > $limit {
            cell.add_attribute(Attribute::Bold)
        } else {
            cell
        }
    }};
}

fn main() -> Result<(), StatusError> {
    let opts: Opts = Opts::parse();
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = stdout.lock(); // acquire a lock on it
    loop {
        let localtime: DateTime<Local> = Local::now();
        let mut table = Table::new();

        table
            .load_preset("     ═  |          ")
            .set_content_arrangement(ContentArrangement::Dynamic);

        table.enforce_styling();

        let nvml = Nvml::init()?;
        let device_num = nvml.device_count()?;

        let system = System::new_with_specifics(
            RefreshKind::new()
                .with_processes(ProcessRefreshKind::new().with_user())
                .with_users_list(),
        );

        for index in 0..device_num {
            let device = nvml.device_by_index(index)?;
            let device_name = device.name()?;
            let device_memory = device.memory_info()?;
            let device_processes = device.running_compute_processes()?;

            let mut process_info = vec![];
            for device_process in device_processes {
                let process = system.process(Pid::from_u32(device_process.pid)).unwrap();
                let user_id = process.user_id().expect("Unable to get UID!");
                let user = User::from_uid(Uid::from(*user_id.to_owned()))?.unwrap();
                let used = match device_process.used_gpu_memory {
                    UsedGpuMemory::Unavailable => String::from("Unavailable"),
                    UsedGpuMemory::Used(m) => {
                        format!("{}M", m >> 20)
                    }
                };

                let info = {
                    let mut s = user.name;
                    s = s + ":" + &process.cmd().join(" ");
                    s = s + "/" + &device_process.pid.to_string();
                    s
                };
                process_info.push(format!("{}({})", info, used));
            }

            let temperature = device.temperature(TemperatureSensor::Gpu)?; // 50
            let util_rates = device.utilization_rates()?.gpu; // 30

            let device_memory_rates = device_memory.used as f64 / device_memory.total as f64; // 50

            let temperature_cell = bold_limit!(temperature, 50, Color::Red, "{}°C", temperature);
            let utilization_cell = bold_limit!(util_rates, 30, Color::Green, "{} %", util_rates);

            let mut row = vec![
                Cell::new(format!("[{}]", index)).fg(Color::DarkCyan), // index
                Cell::new(device_name).fg(Color::DarkBlue),            // gpu type name
                temperature_cell,
                utilization_cell,
            ];

            let fan_color = Color::Rgb {
                r: 255,
                g: 0,
                b: 255,
            };
            let fan_rates = device.fan_speed(0)?; // 50
            let fan_cell = bold_limit!(fan_rates, 50, fan_color, "F: {} %", fan_rates);
            row.push(fan_cell);

            let en_util_rates = device.encoder_utilization()?.utilization; // 30
            let de_util_rates = device.decoder_utilization()?.utilization; // 30

            let encoder_cell =
                bold_limit!(en_util_rates, 30, Color::Cyan, "E: {} %", en_util_rates);
            let decoder_cell =
                bold_limit!(de_util_rates, 30, Color::Cyan, "D: {} %", de_util_rates);

            row.push(encoder_cell);
            row.push(decoder_cell);

            let pow_usage = device.power_usage()?;
            let pow_limit = device.power_management_limit()?;
            let pow_rates = pow_usage as f32 / pow_limit as f32; // 50
            let pow_cell = bold_limit!(
                pow_rates,
                0.5,
                Color::DarkMagenta,
                "{} / {} W",
                pow_usage / 1000,
                pow_limit / 1000
            );
            let memory_cell = bold_limit!(
                device_memory_rates,
                0.5,
                Color::Yellow,
                "{} / {} MB",
                device_memory.used >> 20,
                device_memory.total >> 20
            );

            row.push(pow_cell);
            row.push(memory_cell);
            row.push(Cell::new(process_info.join(",")).fg(Color::DarkYellow));

            table.add_row(row);
        }
        if !opts.continuous {
            println!(
                "{}\t{}\t{}",
                hostname::get()?.to_str().unwrap_or_default(),
                localtime.format("%Y-%m-%d %H:%M:%S"),
                nvml.sys_driver_version()?,
            );
            println!("{}", table);
            break;
        } else {
            writeln!(
                handle,
                "{}\t{}\t{}\n{}",
                hostname::get()?.to_str().unwrap_or_default(),
                localtime.format("%Y-%m-%d %H:%M:%S"),
                nvml.sys_driver_version().unwrap_or_default(),
                table
            )?;
            handle.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(2));
            print!("\x1B[2J\x1B[H");
        }
    }
    Ok(())
}
