use std::{
    io::{self, BufReader, BufWriter, ErrorKind, stdout},
    path::{Path, PathBuf},
    process::exit,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use clap::Parser;
use color_eyre::eyre::eyre;
use dialoguer::{
    Select,
    console::{Color, Style, Term},
    theme::ColorfulTheme,
};
use notify::{Config, EventKind, PollWatcher, RecursiveMode, Watcher};
use serialport::SerialPort;

static DIALOGUING: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
struct Args {
    path: Option<PathBuf>,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install().unwrap();

    let _ = ctrlc::set_handler(move || {
        if !DIALOGUING.load(Ordering::Relaxed) {
            println!();
            exit(130);
        }
    });

    let args = Args::parse();

    let selected_port = match args.path {
        Some(path) => path.to_string_lossy().into_owned(),
        None => {
            let ports = serialport::available_ports()?;

            let port_names = ports
                .iter()
                .map(|p| {
                    let port_name = p.port_name.as_str();
                    let usb_data = match &p.port_type {
                        serialport::SerialPortType::UsbPort(usb_port_info) => {
                            match (
                                usb_port_info.product.as_deref(),
                                usb_port_info.manufacturer.as_deref(),
                            ) {
                                (None, None) => "".to_string(),
                                (None, Some(manufacturer)) => format!(" - {manufacturer}"),
                                (Some(product), None) => format!(" - {product}"),
                                (Some(product), Some(manufacturer)) => {
                                    format!(" - {product} {manufacturer}")
                                }
                            }
                        }
                        serialport::SerialPortType::PciPort => "".into(),
                        serialport::SerialPortType::BluetoothPort => "".into(),
                        serialport::SerialPortType::Unknown => "".into(),
                    };

                    format!("{port_name}{}", usb_data)
                })
                .collect::<Vec<_>>();

            let theme = ColorfulTheme {
                active_item_style: Style::new().fg(Color::Color256(202)),
                ..Default::default()
            };

            DIALOGUING.store(true, Ordering::Relaxed);
            match Select::with_theme(&theme)
                .with_prompt("Pick your serial port")
                .default(0)
                .items(&port_names)
                .interact()
            {
                Ok(selection) => {
                    DIALOGUING.store(false, Ordering::Relaxed);
                    ports[selection].port_name.clone()
                }
                Err(_) => {
                    DIALOGUING.store(false, Ordering::Relaxed);
                    Term::stderr().show_cursor().ok();
                    return Ok(());
                }
            }
        }
    };

    let mut stdout = BufWriter::with_capacity(1024 * 1024, stdout().lock());
    let mut port = open_port(&selected_port, 9600)?;
    eprintln!("[picocom] Connected to {}", selected_port);

    loop {
        match io::copy(&mut port, &mut stdout) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::TimedOut => {}
            Err(err)
                if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::BrokenPipe =>
            {
                eprintln!("[picocom] Lost connection to {}: {err}", selected_port,);
                wait_for_creation(&selected_port)?;
                eprintln!("[picocom] Reconnected to {}", selected_port);
                port = open_port(&selected_port, 9600)?;
            }
            Err(err) => return Err(err.into()),
        };
    }
}

fn open_port(path: &str, baud_rate: u32) -> color_eyre::Result<BufReader<Box<dyn SerialPort>>> {
    Ok(BufReader::with_capacity(
        1024 * 1024,
        serialport::new(path, baud_rate)
            .timeout(Duration::MAX)
            .open()?,
    ))
}

fn wait_for_creation<P: AsRef<Path>>(path: P) -> color_eyre::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // Have to use poll watcher because for some reason notification watcher doesn't work for /dev on macos
    let mut watcher = PollWatcher::new(
        tx,
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )?;

    let path = path.as_ref();
    watcher.watch(path.parent().unwrap(), RecursiveMode::NonRecursive)?;

    if path.exists() {
        return Ok(());
    }

    for res in rx {
        let event = res?;
        if let EventKind::Create(_) = event.kind {
            // Check if the event matches our target file
            if event.paths.contains(&path.to_path_buf()) {
                return Ok(());
            }
        }
    }

    Err(eyre!("idk what happened lol :P"))
}
