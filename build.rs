use version_check::Version;
use static_files::resource_dir;

/// Get a list of the current features enabled.
macro_rules! get_features {
    ($($feature:expr),*) => {
        vec![
            $(
                #[cfg(feature = $feature)]
                $feature,
            )*
        ]
    }
}

fn main() {
    let version = match version_check::triple() {
        Some((ver, ..)) => ver.to_mmp(),
        None => Version::parse("1.0.0").unwrap().to_mmp(),
    };

    if version.0 != 1 && version.1 < 62 {
        panic!("Minimum rust version required is nightly 1.62, please update your rust version via `rustup update`.");
    }

    let features = get_features!(
        "fs_storage",
        "cloudinary_storage",
        "s3_storage",
        "tixte_storage",
        "sled_storage"
    );

    if features.is_empty() {
        panic!("You must set a storage provider.");
    }

    if features.len() > 1 {
        panic!(
            "\
            You can only have one storage provider.
            Provided Features: {}
        ",
            features.join(", ")
        );
    }

    resource_dir("./static").build().expect("Failed packing static files");
}
