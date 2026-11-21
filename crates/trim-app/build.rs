fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../resources/icons/trim.ico");
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../resources/icons/trim.ico");
        res.set("ProductName", "triM.");
        res.set("FileDescription", "triM. Desktop Video Editor");
        res.set("LegalCopyright", "Copyright (c) triM. contributors");
        res.compile()?;
    }
    Ok(())
}
