use std::{collections::{HashMap, HashSet}, io::Write, sync::Arc};

use proc_macro2::{Ident, Literal, Span, TokenStream};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;

const LATEST_VERSION: &'static str = "2024.3.2";

#[derive(Debug, Deserialize)]
struct FolderItem {
    name: String,
    folder: bool,
}

#[derive(Debug, Deserialize)]
struct Folder {
    path: String,
    data: Vec<FolderItem>,
}

#[derive(Debug, Default)]
struct State {
    jni: JNIState,
    cpp: CppState,
    java: JavaState,
}

#[derive(Debug, Default)]
struct JNIState {
    items: HashMap<(String, String), HashSet<BinaryPlatform>>
}

#[derive(Debug, Default)]
struct CppState {
    items: HashMap<(String, String), HashSet<BinaryPlatform>>
}

#[derive(Debug, Default)]
struct JavaState {
    items: HashSet<(String, String)>
}

async fn index_artifactory(client: &Client, link: &str, state: Arc<Mutex<State>>) {
    let index = client.get(link).send().await.unwrap();
    if !index.headers().get("Content-Type").and_then(|x| x.to_str().ok()).map(|x| x.starts_with("application/json")).unwrap_or(false) {
        return;
    }
    let index: Folder = index.json().await.unwrap();
    let jni_end = format!("-jni/{}", LATEST_VERSION);
    let java_end = format!("-java/{}", LATEST_VERSION);
    let cpp_end = format!("-cpp/{}", LATEST_VERSION);
    for item in index.data {
        if item.folder {
            if item.name.starts_with("20") && LATEST_VERSION != item.name {
                continue
            }
            if item.name == "tools" || item.name == "shuffleboard" || item.name.starts_with("javafx") || item.name.starts_with("xrp") || item.name.starts_with("romi") {
                continue
            }
            let next = format!("{}{}/?{}", link.split('?').next().unwrap(), item.name, link.split('?').last().unwrap());
            let state = state.clone();
            Box::pin(async move { index_artifactory(client, &next, state).await }).await;
        } else {
            if index.path.ends_with(&jni_end) {
                let name = item.name;
                let parts = index.path.split('/').collect::<Vec<_>>();
                let (_, rest) = parts.split_last().unwrap();
                let (item, group) = rest.split_last().unwrap();
                let group = group.join(".");
                let ty = name[item.len() + LATEST_VERSION.len() + 2..].split('.').next().unwrap();
                match BinaryPlatform::from_str(ty) {
                    Some(BinaryPlatform::Headers) => {},
                    Some(x) => {
                        let mut state = state.lock().await;
                        let entry = state.jni.items.entry((group, item.to_string())).or_default();
                        entry.insert(x);
                    }
                    None => {},
                }
            } else if index.path.ends_with(&java_end) {
                let name = item.name;
                let parts = index.path.split('/').collect::<Vec<_>>();
                let (_, rest) = parts.split_last().unwrap();
                let (item, group) = rest.split_last().unwrap();
                let group = group.join(".");
                let ty = &name[item.len() + LATEST_VERSION.len() + 2..];
                if ty == "jar" {
                    let mut state = state.lock().await;
                        state.java.items.insert((group, item.to_string()));
                }
            } else if index.path.ends_with(&cpp_end) {
                let name = item.name;
                let parts = index.path.split('/').collect::<Vec<_>>();
                let (_, rest) = parts.split_last().unwrap();
                let (item, group) = rest.split_last().unwrap();
                let group = group.join(".");
                let ty = name[item.len() + LATEST_VERSION.len() + 2..].split('.').next().unwrap();
                match BinaryPlatform::from_str(ty) {
                    Some(BinaryPlatform::Headers) => {},
                    Some(x) => {
                        let mut state = state.lock().await;
                        let entry = state.cpp.items.entry((group, item.to_string())).or_default();
                        entry.insert(x);
                    }
                    None => {},
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let state = Arc::new(Mutex::new(State::default()));
    index_artifactory(&client, "https://frcmaven.wpi.edu/ui/api/v1/ui/v2/nativeBrowser/release/?recordNum=0", state.clone()).await;
    let state = state.lock().await;
    let state = &*state;
    let jni: TokenStream = state.jni.items.iter().map(|((group, item), _platforms)| {
        let group_id = Literal::string(group);
        let artifact_id = Literal::string(item);
        quote::quote! {
            JniDependency {
                group_id: #group_id.to_string(),
                artifact_id: #artifact_id.to_string(),
                version: WPILIB_LATEST_VERSION.to_string(),
                is_jar: true,
                sim_mode: None,
                skip_invalid_platforms: false,
                valid_platforms: vec![],
            },
        }.into_iter()
    }).flatten().collect();
    let cpp: TokenStream = state.cpp.items.iter().map(|((group, item), _platforms)| {
        let group_id = Literal::string(group);
        let artifact_id = Literal::string(item);
        quote::quote! {
            CppDependency {
                group_id: #group_id.to_string(),
                artifact_id: #artifact_id.to_string(),
                version: WPILIB_LATEST_VERSION.to_string(),
                header_classifier: "headers".to_string(),
            },
        }.into_iter()
    }).flatten().collect();
    let java: TokenStream = state.java.items.iter().map(|(group, item)| {
        let group_id = Literal::string(group);
        let artifact_id = Literal::string(item);
        quote::quote! {
            JavaDependency {
                group_id: #group_id.to_string(),
                artifact_id: #artifact_id.to_string(),
                version: WPILIB_LATEST_VERSION.to_string(),
            },
        }.into_iter()
    }).flatten().collect();
    let mut f = std::fs::File::create("src/wpilib.rs").unwrap();
    writeln!(&mut f, "{}", quote::quote! {
        #[doc = "Create a [`VendorDep`] that includes all libraries that come with WPILib."]
            pub fn wpilib_as_a_vendordep() -> VendorDep {
                VendorDep {
                    file_name: "".to_string(),
                    name: "".to_string(),
                    version: "".to_string(),
                    frc_year: 2024,
                    uuid: "".to_string(),
                    maven_urls: vec!["https://frcmaven.wpi.edu/artifactory/release/".to_string()],
                    json_url: "".to_string(),
                    conflicts_with: vec![],
                    java_dependencies: vec![
                        #java
                    ],
                    jni_dependencies: vec![
                        #jni
                    ],
                    cpp_dependencies: vec![
                        #cpp
                    ],
                }
            }
    }).unwrap();
    drop(f);
    std::process::Command::new("rustfmt").arg("src/wpilib.rs").spawn().unwrap().wait().unwrap();
}

macro_rules! binary_platform {
    ($name:ident {$($variant:ident = $val:literal),* $(,)?}) => {
        #[doc = "Valid platforms for WPILib execution."]
        #[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $(
                #[serde(rename = $val)]
                $variant,
            )*
        }

        impl $name {
            pub fn to_str(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $val,
                    )*
                }
            }

            pub fn from_str(input: &str) -> Option<Self> {
                match input {
                    $(
                        $val => Some(Self::$variant),
                    )*
                    _ => None
                }
            }
        }
    };
}

binary_platform!(BinaryPlatform {
    LinuxArm32 = "linuxarm32",
    LinuxArm64 = "linuxarm64",
    LinuxAthena = "linuxathena",
    LinuxX86_64 = "linuxx86-64",
    OsxUniversal = "osxuniversal",
    WindowsArm64 = "windowsarm64",
    WindowsX86_64 = "windowsx86-64",
    Headers = "headers",
});
