//! The dimension this crate is: not a feature, so it can't be turned off.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(dim, values(\"2d\", \"3d\"))");
    println!("cargo::rustc-cfg=dim=\"2d\"");
}
