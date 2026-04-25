use std::fs::File;
use std::io::Write;

pub fn export_svg_dashboard(path: &str, svg_paths: &[String]) -> anyhow::Result<()> {
    let mut file = File::create(path)?;

    writeln!(file, "<html><body>")?;

    for svg in svg_paths {
        writeln!(file, "<h2>{}</h2>", svg)?;
        writeln!(file, "<object type=\"image/svg+xml\" data=\"{}\"></object>", svg)?;
        writeln!(file, "<hr>")?;
    }

    writeln!(file, "</body></html>")?;
    Ok(())
}
