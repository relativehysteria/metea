#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Initialize the logcat backend.
    let logger = android_logger::Config::default()
        .with_max_level(log::LevelFilter::Info);
    android_logger::init_once(logger);

    // Save the path to the internal app storage. We will pass it to the app and
    // use it to save data.
    let internal_storage = app.internal_data_path();

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    eframe::run_native(
        "metea",
        options,
        Box::new(|cc| Ok(Box::new(crate::App::new(cc, internal_storage)))),
    ).unwrap()
}
