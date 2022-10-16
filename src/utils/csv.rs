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

#[test]
fn validate_csv_generation() {
    let s = ScaledResources {
        name: "test-kuber2-deploy1".to_string(),
        namespace: "kuber1".to_string(),
        kind: crate::Resources::Deployment,
    };
    let c = generate_csv(&[s], "file");
    assert_eq!(c.unwrap(), ());
    assert_eq!(std::path::Path::new("/tmp/file.csv").exists(), true);
    std::fs::remove_file(format!("/tmp/file.csv")).unwrap()
}
