use log::info;

use crate::{downscaler::ScaledResources, error::Error};
// TODO,use async io
pub fn generate_csv(resources: &[ScaledResources], file_name: &str) -> Result<(), Error> {
    let mut wtr = csv::Writer::from_path(format!("/tmp/{}.csv", file_name))?;
    for r in resources {
        wtr.write_record([
            r.kind.to_string(),
            r.namespace.to_string(),
            r.name.to_string(),
        ])?;
    }
    wtr.flush()?;
    info!(
        "{} written at location {}",
        file_name,
        std::env::current_dir()?.as_os_str().to_str().unwrap() // TODO fix this
    );
    Ok(())
}
