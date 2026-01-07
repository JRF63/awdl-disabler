fn main() {
    println!("cargo:rerun-if-changed=os_log/main.c");
    cc::Build::new().file("os_log/main.c").compile("os_log");
}
