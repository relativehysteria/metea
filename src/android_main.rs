use crate::Storage;

#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Initialize the logcat backend.
    let logger = android_logger::Config::default()
        .with_max_level(log::LevelFilter::Info);
    android_logger::init_once(logger);

    // Save the path to the internal app storage. We will pass it to the app and
    // use it to save data.
    let storage = app.internal_data_path()
        .map(|path| Storage::new(path))
        .expect("Couldn't get internal storage to application");

    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        ..Default::default()
    };

    // Create the platform
    let platform = crate::Platform { app, storage };

    eframe::run_native(
        "metea",
        options,
        Box::new(|_cc| Ok(Box::new(crate::App::new(platform)))),
    ).unwrap()
}
