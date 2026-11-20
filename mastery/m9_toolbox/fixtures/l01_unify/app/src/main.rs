fn main() {
    println!(
        "app: {} / build script: {}",
        shared::features(),
        env!("SHARED_IN_BUILD_SCRIPT")
    );
}
