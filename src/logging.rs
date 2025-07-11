pub fn setup_logging() {
    tui_logger::init_logger(tui_logger::LevelFilter::Trace).unwrap();

    // Create tmp dir if it does not exist
    std::fs::create_dir_all("tmp").unwrap_or_else(|_| {
        eprintln!("Failed to create tmp directory, logging will not work properly.");
    });
    let file_config = tui_logger::TuiLoggerFile::new("tmp/app.log");
    tui_logger::set_log_file(file_config);

    tui_logger::set_default_level(tui_logger::LevelFilter::Trace);
}
