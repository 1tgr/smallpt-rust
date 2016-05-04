extern crate pkg_config;

fn main() {
    pkg_config::probe_library("gtk+-3.0").unwrap();
}
