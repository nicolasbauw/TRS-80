use directories::UserDirs;
use fs_extra::{copy_items, dir::create};

fn main() {
    if let Some(user_dirs) = UserDirs::new() {
        let home_dir = user_dirs.home_dir();
        let mut dest = home_dir.to_path_buf();

        // Do not overwrite config file if present
        let options = fs_extra::dir::CopyOptions::new().skip_exist(true).overwrite(false);

        // Create .config directory in user's home directory
        dest.push(".config/");
        create(&dest, false).unwrap_or_default(); // Create error ? directory exists, so we don't care

        // Create trust-80's config directory in user's home directory
        dest.push("trust80/");
        if create(&dest, false).is_err() {
            println!("~/.config/trust80 already exists")
        };

        // Copy teletype/config.toml to home_dir/.config/teletype/config.toml
        let from_paths = vec!["config/config.toml"];
        println!("Copying {:#?} to {:#?}", from_paths, dest);
        copy_items(&from_paths, dest, &options).unwrap();
    }
}
