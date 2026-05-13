fn main() {
    // Get the path to the data local storage.
    let storage_dir = dirs::data_local_dir()
        .expect("Couldn't find the directory for permanent storage")
        .join("metea");

    // Create the storage.
    let storage = metea::Storage::new(storage_dir)
        .expect("Couldn't create storage state");

    // Create the platform specific struct and run the app!
    let platform = metea::Platform { storage };
    platform.run_native().expect("Application returned error");
}
