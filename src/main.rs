use ctrlc;
use hawktracer_converter_lib as hcl;
use indicatif;
use indicatif::{ProgressBar, ProgressStyle};

fn create_output_path(path: &str) -> String {
    let now = chrono::Local::now();
    now.format(path).to_string()
}

fn create_label_getter(values: Option<clap::Values>) -> hcl::LabelGetter {
    let mut map = hcl::LabelMap::new();

    if let Some(values) = values {
        for value in values {
            match map.load_from_file(value) {
                Ok(_) => eprintln!("Successfully loaded mapping from {}", value),
                Err(err) => eprintln!("Failed to load mapping from {}. Error: {}", value, err),
            };
        }
    }

    hcl::LabelGetter::new(map, vec!["label".to_owned(), "name".to_owned()])
}

fn create_output_stream(is_stdout: bool, output_file: &str) -> Box<dyn std::io::Write> {
    if is_stdout {
        return Box::new(std::io::stdout());
    }

    let output_path = create_output_path(output_file);
    eprintln!("Data will be saved at: {:?}", output_path);
    Box::new(
        std::fs::File::create(&output_path)
            .unwrap_or_else(|_| panic!("Can't create output file {}", output_path)),
    )
}

fn wait_for_connection(socket_addr: std::net::SocketAddr) -> std::io::Result<Box<dyn std::io::Read>> {
    use std::io::ErrorKind;
    loop {
        let tcp_stream = std::net::TcpStream::connect(socket_addr);

        match tcp_stream {
            Ok(tcp_stream) => return Ok(Box::new(tcp_stream)),
            Err(err) => match err.kind() {
                ErrorKind::ConnectionReset
                | ErrorKind::BrokenPipe
                | ErrorKind::AddrNotAvailable
                | ErrorKind::ConnectionAborted => {
                    eprintln!("Connection error: {:?}", err);
                    return Err(err);
                }
                _ => {}
            },
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new(30);
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("/—\\| ")
        .template("{prefix:.bold.dim} {spinner:.green} {wide_msg}");
    pb.set_style(spinner_style);
    pb.set_message(message);
    pb.enable_steady_tick(200);
    pb
}

fn create_event_reader(source: &str) -> std::io::Result<hawktracer_parser::reader::EventReader> {
    let source_obj: Box<dyn std::io::Read> =
        if let Ok(ip_address) = source.parse::<std::net::Ipv4Addr>() {
            wait_for_connection(std::net::SocketAddr::new(
                std::net::IpAddr::V4(ip_address),
                8765,
            ))?
        } else if let Ok(ip_address) = source.parse::<std::net::SocketAddr>() {
            wait_for_connection(ip_address)?
        } else {
            Box::new(std::fs::File::open(source)?)
        };

    let provider = hawktracer_parser::data_provider::DataProvider::new(source_obj);
    Ok(hawktracer_parser::reader::EventReader::new(provider))
}

fn setup_stop_handler() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    running
}

fn main() {
    let converter_manager = hcl::ConverterManager::new();

    let matches = clap::App::new("hawktracer-converter")
        .about("Converts HawkTracer data stream to well-known data formats")
        .author("Marcin Kolny <marcin.kolny@gmail.com>")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            clap::Arg::with_name("source")
                .long("source")
                .short("s")
                .help("Data source description (either filename, or server address)")
                .required(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("format")
                .long("format")
                .short("f")
                .takes_value(true)
                .required(true)
                .possible_values(&converter_manager.get_converters()[..])
                .default_value(converter_manager.get_converters()[0])
                .help("Conversion format"),
        )
        .arg(
            clap::Arg::with_name("output-file")
                .long("output-file")
                .short("o")
                .takes_value(true)
                .required_unless("stdout")
                .help("Output file")
                .default_value("hawktracer-trace-%Y-%m-%d-%H_%M_%S.httrace"),
        )
        .arg(
            clap::Arg::with_name("stdout")
                .long("stdout")
                .help("Print data to standard output"),
        )
        .arg(
            clap::Arg::with_name("map-files")
                .long("map-files")
                .short("m")
                .min_values(1)
                .help("List of mapping files"),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Print debug information"),
        )
        .get_matches();

    let is_verbose = matches.is_present("verbose");
    let source = matches.value_of("source").unwrap();

    let mut converter = converter_manager
        .create_converter(
            matches.value_of("format").unwrap(),
            create_output_stream(
                matches.is_present("stdout"),
                matches.value_of("output-file").unwrap(),
            ),
            create_label_getter(matches.values_of("map-files")),
        )
        .expect("Unable to create converter");

    let connection_spinner = create_spinner(&format!(
        "Waiting for connection to source: {}",
        &source[..]
    ));

    let mut reader = create_event_reader(source)
        .unwrap_or_else(|_| panic!("Unable to create reader from source: {}", &source[..]));
    connection_spinner.finish_with_message(&format!("Connected to source: {}", &source[..]));
    let mut reg = hawktracer_parser::EventKlassRegistry::new();

    let data_read_spinner =
        create_spinner(&format!("{}", "Getting data. Press [Ctrl+C to finish]"));

    let running_flag = setup_stop_handler();

    while running_flag.load(std::sync::atomic::Ordering::SeqCst) {
        match reader.read_event(&mut reg) {
            Ok(event) => {
                if let Err(err) = converter.process_event(&event.flat_event(), &reg) {
                    // TODO flat optional from command line
                    if is_verbose {
                        eprintln!("Error processing event: {}", err);
                    }
                }
            }
            Err(_) => break,
        }
    }

    data_read_spinner.finish_with_message("Done tracing!");
}
