use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../build.env");
    let dotenv_path = Path::new("../build.env");
    let dotenv_contents = if dotenv_path.exists() {
        let contents = fs::read_to_string(dotenv_path).unwrap();
        let mut res = HashMap::new();
        for line in contents.lines() {
            if line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once('=').unwrap();
            res.insert(key.to_string(), value.to_string());
        }
        res
    } else {
        HashMap::new()
    };

    let get_env = move |key: &str| {
        env::var(key)
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| dotenv_contents.get(key).cloned())
            .filter(|value| !value.is_empty())
    };

    let build_envs = ["LAUNCHER_NAME", "VERSION_MANIFEST_URL"];

    let optional_envs = ["BACKEND_API_BASE", "VERSION"];

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = format!("{out_dir}/generated.rs");

    let mut config_content = String::new();
    for env in build_envs.iter() {
        let value = get_env(env).unwrap_or_else(|| panic!("{env} is not set"));
        config_content.push_str(&format!("pub const {env}: &str = \"{value}\";\n"));
    }
    for env in optional_envs.iter() {
        match get_env(env) {
            Some(value) => {
                config_content.push_str(&format!(
                    "pub const {env}: Option<&str> = Some(\"{value}\");\n"
                ));
            }
            None => {
                config_content.push_str(&format!("pub const {env}: Option<&str> = None;\n"));
            }
        }
    }
    let use_native_glfw_default = get_env("USE_NATIVE_GLFW_DEFAULT")
        .unwrap_or_else(|| "false".to_string())
        .parse::<bool>()
        .expect("USE_NATIVE_GLFW_DEFAULT must be a boolean");
    config_content.push_str(&format!(
        "pub const USE_NATIVE_GLFW_DEFAULT: bool = {use_native_glfw_default};\n"
    ));
    fs::write(dest_path, config_content).unwrap();

    let icon_path = format!(
        "{}/assets/icon.png",
        env::var("CARGO_MANIFEST_DIR").unwrap().replace("\\", "/"),
    );
    let icon_out_file = format!("{out_dir}/icon_file_bytes.rs");
    let icon_value = if Path::new(&icon_path).exists() {
        format!("Some(include_bytes!(\"{icon_path}\"))")
    } else {
        "None".to_string()
    };
    fs::write(
        &icon_out_file,
        format!("pub const LAUNCHER_ICON: Option<&[u8]> = {icon_value};"),
    )
    .unwrap();

    #[cfg(target_os = "windows")]
    {
        let windows_icon_path = "assets/icon.ico";
        if Path::new(windows_icon_path).exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(windows_icon_path);
            res.compile().unwrap();
        }
    }
}
