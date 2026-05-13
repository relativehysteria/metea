#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Initialize the logcat backend.
    let logger = android_logger::Config::default()
        .with_max_level(log::LevelFilter::Info);
    android_logger::init_once(logger);

    // Get path to the internal storage for this application.
    let storage_dir = app.internal_data_path()
        .expect("Couldn't get internal storage to application");

    // Create the storage.
    let storage = crate::Storage::new(storage_dir)
        .expect("Couldn't create storage state");

    // Create the platform specific struct and run the app!
    let platform = crate::Platform { app, storage };
    platform.run_native().expect("Application returned error");
}
